//! Long-running maintenance commands (`sfc`, `dism`, `defrag`, `chkdsk`).
//!
//! Each job runs in its own thread, keeps the last [`MAX_LINES`] output lines in
//! memory and is polled by the UI. Only one job runs at a time: these tools
//! fight over the same disk, and starting two at once is never what the user
//! meant.

use std::io::{BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Output lines kept per job. Enough to follow the last phase of a scan.
pub const MAX_LINES: usize = 400;
/// Finished jobs remembered for the "reopen the page" case.
const MAX_JOBS: usize = 12;
/// How often the supervisor checks whether the child exited.
const POLL: Duration = Duration::from_millis(150);

/// One maintenance command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// `sfc /scannow`.
    Sfc,
    /// `DISM /Online /Cleanup-Image /RestoreHealth`.
    Dism,
    /// Traditional defragmentation.
    Defrag,
    /// Retrim of the free space (SSD).
    Trim,
    /// Online read-only filesystem scan.
    CheckDisk,
}

impl JobKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "sfc" => Some(Self::Sfc),
            "dism" => Some(Self::Dism),
            "defrag" => Some(Self::Defrag),
            "trim" => Some(Self::Trim),
            "check-disk" => Some(Self::CheckDisk),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sfc => "sfc",
            Self::Dism => "dism",
            Self::Defrag => "defrag",
            Self::Trim => "trim",
            Self::CheckDisk => "check-disk",
        }
    }

    /// Name shown in the console header.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sfc => "Comprobar archivos del sistema (SFC)",
            Self::Dism => "Reparar imagen de Windows (DISM)",
            Self::Defrag => "Desfragmentar discos",
            Self::Trim => "Optimizar SSD (TRIM)",
            Self::CheckDisk => "Analizar el disco del sistema (CHKDSK)",
        }
    }

    /// Exact command line, also used by the tests to document it.
    pub fn argv(self) -> Vec<String> {
        match self {
            Self::Sfc => vec!["sfc.exe".into(), "/scannow".into()],
            Self::Dism => vec![
                "DISM.exe".into(),
                "/Online".into(),
                "/Cleanup-Image".into(),
                "/RestoreHealth".into(),
                "/NoRestart".into(),
            ],
            Self::Defrag => vec![
                "defrag.exe".into(),
                "/C".into(),
                "/D".into(),
                "/U".into(),
                "/V".into(),
            ],
            Self::Trim => vec![
                "defrag.exe".into(),
                "/C".into(),
                "/L".into(),
                "/U".into(),
                "/V".into(),
            ],
            Self::CheckDisk => vec![
                "chkdsk.exe".into(),
                system_drive(),
                "/scan".into(),
                "/perf".into(),
            ],
        }
    }

    fn command(self) -> Command {
        let argv = self.argv();
        let mut command = Command::new(&argv[0]);
        command.args(&argv[1..]);
        command
    }
}

fn system_drive() -> String {
    std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string())
}

/// State of a job as the UI sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Serializable job snapshot.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub status: JobStatus,
    pub started_ms: u64,
    pub finished_ms: Option<u64>,
    pub exit_code: Option<i32>,
    /// Oldest first; the tail is what the console shows.
    pub lines: Vec<String>,
    /// Lines dropped because the buffer was full.
    pub dropped_lines: u64,
}

struct JobState {
    snapshot: JobSnapshot,
    child: Option<Child>,
    cancel_requested: bool,
}

/// Owns the running job and the short history of finished ones.
#[derive(Default)]
pub struct JobManager {
    jobs: Mutex<Vec<Arc<Mutex<JobState>>>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a job, or explains why it cannot start.
    pub fn start(&self, kind: JobKind) -> Result<JobSnapshot, String> {
        if let Some(running) = self.running() {
            return Err(format!("ya hay una tarea en curso: {}", running.label));
        }

        let mut command = kind.command();
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;

            // CREATE_NO_WINDOW: the console windows would otherwise flash.
            command.creation_flags(0x0800_0000);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("no se pudo iniciar {}: {error}", kind.label()))?;

        let started_ms = now_ms();
        let state = Arc::new(Mutex::new(JobState {
            snapshot: JobSnapshot {
                id: format!("job-{started_ms}"),
                kind: kind.as_str().to_string(),
                label: kind.label().to_string(),
                status: JobStatus::Running,
                started_ms,
                finished_ms: None,
                exit_code: None,
                lines: Vec::new(),
                dropped_lines: 0,
            },
            child: None,
            cancel_requested: false,
        }));

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        {
            let mut guard = lock(&state);
            guard.child = Some(child);
        }

        let mut readers = Vec::new();
        if let Some(stdout) = stdout {
            readers.push(spawn_reader(stdout, Arc::clone(&state)));
        }
        if let Some(stderr) = stderr {
            readers.push(spawn_reader(stderr, Arc::clone(&state)));
        }

        let supervisor = Arc::clone(&state);
        std::thread::Builder::new()
            .name("wexo-tool-job".to_string())
            .spawn(move || supervise(supervisor, readers))
            .map_err(|error| format!("no se pudo supervisar la tarea: {error}"))?;

        let snapshot = lock(&state).snapshot.clone();
        tracing::info!(job = %snapshot.id, tool = %snapshot.kind, "maintenance job started");
        self.remember(state);
        Ok(snapshot)
    }

    pub fn get(&self, id: &str) -> Option<JobSnapshot> {
        lock(&self.jobs)
            .iter()
            .find(|job| lock(job).snapshot.id == id)
            .map(|job| lock(job).snapshot.clone())
    }

    pub fn all(&self) -> Vec<JobSnapshot> {
        lock(&self.jobs)
            .iter()
            .map(|job| lock(job).snapshot.clone())
            .collect()
    }

    pub fn running(&self) -> Option<JobSnapshot> {
        self.all()
            .into_iter()
            .find(|job| job.status == JobStatus::Running)
    }

    /// Asks the job to stop. Returns `false` when it was not running.
    pub fn cancel(&self, id: &str) -> bool {
        let Some(job) = lock(&self.jobs)
            .iter()
            .find(|job| lock(job).snapshot.id == id)
            .cloned()
        else {
            return false;
        };

        let mut guard = lock(&job);
        if guard.snapshot.status != JobStatus::Running {
            return false;
        }
        guard.cancel_requested = true;
        if let Some(child) = guard.child.as_mut() {
            let _ = child.kill();
        }
        tracing::info!(job = %id, "maintenance job cancelled by user");
        true
    }

    /// Keeps the newest [`MAX_JOBS`], dropping the oldest finished ones.
    fn remember(&self, job: Arc<Mutex<JobState>>) {
        let mut jobs = lock(&self.jobs);
        jobs.push(job);
        while jobs.len() > MAX_JOBS {
            let position = jobs
                .iter()
                .position(|job| lock(job).snapshot.status != JobStatus::Running);
            match position {
                Some(index) => {
                    jobs.remove(index);
                }
                None => break,
            }
        }
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    stream: R,
    state: Arc<Mutex<JobState>>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("wexo-tool-reader".to_string())
        .spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut chunk = [0u8; 4096];
            let mut pending: Vec<u8> = Vec::new();

            loop {
                let read = match reader.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(read) => read,
                    Err(_) => break,
                };

                for &byte in &chunk[..read] {
                    // Console tools overwrite progress with `\r`, so both
                    // terminators end a line.
                    if byte == b'\n' || byte == b'\r' {
                        push_line(&state, &pending);
                        pending.clear();
                    } else {
                        pending.push(byte);
                    }
                    // A tool that never emits a terminator must not grow the
                    // buffer without bound.
                    if pending.len() >= 4096 {
                        push_line(&state, &pending);
                        pending.clear();
                    }
                }
            }

            push_line(&state, &pending);
        })
        .expect("failed to spawn output reader")
}

fn push_line(state: &Arc<Mutex<JobState>>, raw: &[u8]) {
    if raw.is_empty() {
        return;
    }
    let text = strip_ansi(&decode_console(raw));
    let text = text.trim();
    if text.is_empty() {
        return;
    }

    let mut guard = lock(state);
    if guard.snapshot.lines.len() >= MAX_LINES {
        guard.snapshot.lines.remove(0);
        guard.snapshot.dropped_lines += 1;
    }
    guard.snapshot.lines.push(text.to_string());
}

fn supervise(state: Arc<Mutex<JobState>>, readers: Vec<std::thread::JoinHandle<()>>) {
    loop {
        std::thread::sleep(POLL);

        let mut guard = lock(&state);
        if guard.child.is_none() {
            break;
        }
        let outcome = guard.child.as_mut().map(Child::try_wait);

        match outcome {
            Some(Ok(Some(status))) => {
                let cancelled = guard.cancel_requested;
                guard.snapshot.exit_code = status.code();
                guard.snapshot.finished_ms = Some(now_ms());
                guard.snapshot.status = if cancelled {
                    JobStatus::Cancelled
                } else if status.success() {
                    JobStatus::Succeeded
                } else {
                    JobStatus::Failed
                };
                guard.child = None;
                break;
            }
            Some(Ok(None)) => {}
            Some(Err(error)) => {
                guard.snapshot.status = JobStatus::Failed;
                guard.snapshot.finished_ms = Some(now_ms());
                guard
                    .snapshot
                    .lines
                    .push(format!("error al esperar el proceso: {error}"));
                guard.child = None;
                break;
            }
            None => break,
        }
    }

    // Drain whatever the tool wrote before exiting.
    for reader in readers {
        let _ = reader.join();
    }

    let snapshot = lock(&state).snapshot.clone();
    tracing::info!(
        job = %snapshot.id,
        status = ?snapshot.status,
        exit_code = ?snapshot.exit_code,
        "maintenance job finished"
    );
}

/// Console tools write OEM/ANSI bytes; anything that is already valid UTF-8 is
/// used as-is so UTF-8 output is never mangled.
fn decode_console(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }
    oem_to_string(bytes)
}

#[cfg(windows)]
fn oem_to_string(bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_OEMCP};

    if bytes.is_empty() {
        return String::new();
    }
    let length = bytes.len().min(i32::MAX as usize) as i32;

    unsafe {
        let size = MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), length, std::ptr::null_mut(), 0);
        if size <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let mut wide = vec![0u16; size as usize];
        let written = MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), length, wide.as_mut_ptr(), size);
        if written <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        String::from_utf16_lossy(&wide[..written as usize])
    }
}

#[cfg(not(windows))]
fn oem_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Removes ANSI escape sequences (colours, cursor moves) from a console line.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            out.push(character);
            continue;
        }
        match chars.next() {
            // CSI: ESC [ ... final byte in 0x40..=0x7E
            Some('[') => {
                for next in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&next) {
                        break;
                    }
                }
            }
            // OSC: ESC ] ... BEL or ESC \
            Some(']') => {
                while let Some(next) = chars.next() {
                    if next == '\u{7}' {
                        break;
                    }
                    if next == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            // Any other two-character escape.
            Some(_) => {}
            None => {}
        }
    }

    out
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_round_trips_and_has_a_command() {
        for kind in [
            JobKind::Sfc,
            JobKind::Dism,
            JobKind::Defrag,
            JobKind::Trim,
            JobKind::CheckDisk,
        ] {
            assert_eq!(JobKind::parse(kind.as_str()), Some(kind));
            let argv = kind.argv();
            assert!(!argv.is_empty());
            assert!(argv[0].ends_with(".exe"), "{argv:?}");
            assert!(!kind.label().is_empty());
        }
        assert_eq!(JobKind::parse("rm-rf"), None);
    }

    #[test]
    fn trim_and_defrag_use_different_flags() {
        assert!(JobKind::Defrag.argv().contains(&"/D".to_string()));
        assert!(JobKind::Trim.argv().contains(&"/L".to_string()));
    }

    #[test]
    fn ansi_sequences_are_removed() {
        assert_eq!(strip_ansi("\u{1b}[32mOK\u{1b}[0m"), "OK");
        assert_eq!(strip_ansi("plain"), "plain");
        assert_eq!(strip_ansi("\u{1b}]0;title\u{7}text"), "text");
    }

    #[test]
    fn utf8_output_is_kept_and_oem_bytes_are_decoded() {
        assert_eq!(decode_console("Verificación".as_bytes()), "Verificación");
        // 0x82 is 'é' in CP850, which is not valid UTF-8 on its own.
        assert_eq!(decode_console(&[0x56, 0x82]), "Vé");
    }

    #[test]
    fn a_job_that_is_not_running_cannot_be_cancelled() {
        let manager = JobManager::new();
        assert!(!manager.cancel("job-0"));
        assert!(manager.all().is_empty());
        assert!(manager.running().is_none());
    }

    #[test]
    fn snapshots_serialize_with_camel_case_keys() {
        let snapshot = JobSnapshot {
            id: "job-1".to_string(),
            kind: "sfc".to_string(),
            label: "SFC".to_string(),
            status: JobStatus::Running,
            started_ms: 1,
            finished_ms: None,
            exit_code: None,
            lines: vec!["hola".to_string()],
            dropped_lines: 0,
        };
        let json = serde_json::to_string(&snapshot).expect("json");
        assert!(json.contains("\"startedMs\":1"), "{json}");
        assert!(json.contains("\"status\":\"running\""), "{json}");
        assert!(json.contains("\"droppedLines\":0"), "{json}");
    }

    #[test]
    fn line_buffer_keeps_the_tail() {
        let state = Arc::new(Mutex::new(JobState {
            snapshot: JobSnapshot {
                id: "job-1".to_string(),
                kind: "sfc".to_string(),
                label: "SFC".to_string(),
                status: JobStatus::Running,
                started_ms: 0,
                finished_ms: None,
                exit_code: None,
                lines: Vec::new(),
                dropped_lines: 0,
            },
            child: None,
            cancel_requested: false,
        }));

        for index in 0..(MAX_LINES + 5) {
            push_line(&state, format!("linea {index}").as_bytes());
        }

        let guard = lock(&state);
        assert_eq!(guard.snapshot.lines.len(), MAX_LINES);
        assert_eq!(guard.snapshot.dropped_lines, 5);
        let expected = format!("linea {}", MAX_LINES + 4);
        assert_eq!(guard.snapshot.lines.last(), Some(&expected));
    }
}
