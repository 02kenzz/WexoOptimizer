//! Disk cleanup behind the three buttons of the Tools page.
//!
//! Deletion is deliberately conservative: only regular files below a fixed list
//! of well-known folders, never symlinks or junctions, never a folder itself and
//! never anything the system currently has open (those are counted as skipped so
//! the UI can say the truth instead of pretending everything was removed).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Deepest folder level visited below a root. Temporary trees are shallow, and
/// the cap stops a pathological reparse loop from turning a scan into a hang.
const MAX_DEPTH: usize = 12;
/// Upper bound on visited entries, so a huge profile still answers quickly.
const MAX_ENTRIES: usize = 250_000;

/// Which cleanup the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanKind {
    /// `%TEMP%` and the Windows temp folder.
    Temp,
    /// Windows Error Reporting queues and the CBS logs.
    Logs,
    /// Shader and thumbnail caches.
    Cache,
}

impl CleanKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "temp" => Some(Self::Temp),
            "logs" => Some(Self::Logs),
            "cache" => Some(Self::Cache),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Temp => "temp",
            Self::Logs => "logs",
            Self::Cache => "cache",
        }
    }

    /// Folders the kind owns. Missing ones are simply ignored.
    fn roots(self) -> Vec<PathBuf> {
        let mut roots = Vec::new();

        match self {
            Self::Temp => {
                if let Some(temp) = std::env::var_os("TEMP") {
                    roots.push(PathBuf::from(temp));
                }
                if let Some(windows) = std::env::var_os("SystemRoot") {
                    roots.push(PathBuf::from(windows).join("Temp"));
                }
            }
            Self::Logs => {
                for base in [std::env::var_os("LOCALAPPDATA"), std::env::var_os("ProgramData")] {
                    if let Some(base) = base {
                        let wer = PathBuf::from(base)
                            .join("Microsoft")
                            .join("Windows")
                            .join("WER");
                        roots.push(wer.join("ReportQueue"));
                        roots.push(wer.join("ReportArchive"));
                    }
                }
                if let Some(windows) = std::env::var_os("SystemRoot") {
                    roots.push(PathBuf::from(windows).join("Logs").join("CBS"));
                }
            }
            Self::Cache => {
                if let Some(base) = std::env::var_os("LOCALAPPDATA") {
                    let base = PathBuf::from(base);
                    roots.push(base.join("D3DSCache"));
                    roots.push(base.join("NVIDIA").join("DXCache"));
                    roots.push(base.join("NVIDIA").join("GLCache"));
                }
                if let Some(base) = std::env::var_os("ProgramData") {
                    roots.push(
                        PathBuf::from(base)
                            .join("NVIDIA Corporation")
                            .join("NV_Cache"),
                    );
                }
            }
        }

        roots
    }

    /// Folders where only some files belong to this kind (the rest of the
    /// folder must survive).
    fn file_roots(self) -> Vec<PathBuf> {
        match self {
            Self::Cache => std::env::var_os("LOCALAPPDATA")
                .map(|base| {
                    vec![PathBuf::from(base)
                        .join("Microsoft")
                        .join("Windows")
                        .join("Explorer")]
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }
}

/// Files inside a "shared" folder that this kind may delete.
fn belongs(kind: CleanKind, name: &str) -> bool {
    match kind {
        CleanKind::Cache => {
            let lower = name.to_ascii_lowercase();
            (lower.starts_with("thumbcache_") || lower.starts_with("iconcache_"))
                && lower.ends_with(".db")
        }
        _ => false,
    }
}

/// Outcome of a scan or of a cleanup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    /// Bytes found (scan) or freed (cleanup).
    pub bytes: u64,
    /// Files found (scan) or removed (cleanup).
    pub files: u64,
    /// Files that could not be removed because they are in use or protected.
    pub skipped: u64,
    /// Folders that were actually looked at.
    pub roots: Vec<String>,
}

/// Measures how much a cleanup would free without deleting anything.
pub fn scan(kind: CleanKind) -> CleanReport {
    run(kind, false)
}

/// Deletes the files of one kind and reports what actually happened.
pub fn clean(kind: CleanKind) -> CleanReport {
    run(kind, true)
}

fn run(kind: CleanKind, delete: bool) -> CleanReport {
    let mut report = CleanReport::default();
    let mut budget = MAX_ENTRIES;

    let roots = kind.roots();
    for root in &roots {
        if root.is_dir() {
            report.roots.push(root.display().to_string());
        }
    }
    for root in roots.iter().filter(|root| root.is_dir()) {
        visit(root, 0, delete, &mut report, &mut budget);
    }

    for root in kind.file_roots() {
        if !root.is_dir() {
            continue;
        }
        let mut handled = false;
        for entry in fs::read_dir(&root).into_iter().flatten().flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !belongs(kind, &name) {
                continue;
            }
            if !handled {
                report.roots.push(root.display().to_string());
                handled = true;
            }
            handle_file(&entry.path(), delete, &mut report);
        }
    }

    tracing::debug!(
        kind = kind.as_str(),
        delete,
        bytes = report.bytes,
        files = report.files,
        skipped = report.skipped,
        "cleanup finished"
    );
    report
}

fn visit(
    dir: &Path,
    depth: usize,
    delete: bool,
    report: &mut CleanReport,
    budget: &mut usize,
) {
    if depth > MAX_DEPTH {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;

        let path = entry.path();
        // `symlink_metadata` never follows a link, and reparse points are
        // skipped: deleting through a junction would leave the expected folder.
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if is_reparse_point(&meta) {
            continue;
        }

        if meta.is_dir() {
            visit(&path, depth + 1, delete, report, budget);
            if delete {
                // Only removes it when empty; a folder still holding in-use
                // files is left alone.
                let _ = fs::remove_dir(&path);
            }
        } else if meta.is_file() {
            if delete {
                match fs::remove_file(&path) {
                    Ok(()) => {
                        report.bytes += meta.len();
                        report.files += 1;
                    }
                    Err(_) => report.skipped += 1,
                }
            } else {
                report.bytes += meta.len();
                report.files += 1;
            }
        }
    }
}

fn handle_file(path: &Path, delete: bool, report: &mut CleanReport) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    if !meta.is_file() || is_reparse_point(&meta) {
        return;
    }
    if delete {
        match fs::remove_file(path) {
            Ok(()) => {
                report.bytes += meta.len();
                report.files += 1;
            }
            Err(_) => report.skipped += 1,
        }
    } else {
        report.bytes += meta.len();
        report.files += 1;
    }
}

#[cfg(windows)]
fn is_reparse_point(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(meta: &fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn write_file(path: &Path, bytes: usize) {
        let mut file = fs::File::create(path).expect("create");
        file.write_all(&vec![b'x'; bytes]).expect("write");
    }

    #[test]
    fn kinds_round_trip_through_their_names() {
        for kind in [CleanKind::Temp, CleanKind::Logs, CleanKind::Cache] {
            assert_eq!(CleanKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(CleanKind::parse("everything"), None);
    }

    #[test]
    fn scan_counts_without_deleting() {
        let dir = scratch();
        let root = dir.path().join("root");
        fs::create_dir_all(root.join("nested")).expect("dirs");
        write_file(&root.join("a.tmp"), 100);
        write_file(&root.join("nested").join("b.tmp"), 250);

        let report = clean_roots(&[root.clone()], false);

        assert_eq!(report.files, 2);
        assert_eq!(report.bytes, 350);
        assert_eq!(report.skipped, 0);
        assert!(root.join("a.tmp").is_file());
        assert!(root.join("nested").join("b.tmp").is_file());
    }

    #[test]
    fn clean_removes_files_and_empty_folders_but_keeps_the_root() {
        let dir = scratch();
        let root = dir.path().join("root");
        fs::create_dir_all(root.join("nested")).expect("dirs");
        write_file(&root.join("a.tmp"), 10);
        write_file(&root.join("nested").join("b.tmp"), 20);

        let report = clean_roots(&[root.clone()], true);

        assert_eq!(report.files, 2);
        assert_eq!(report.bytes, 30);
        assert!(root.is_dir(), "the root itself must survive");
        assert!(!root.join("nested").exists());
        assert!(!root.join("a.tmp").exists());
    }

    #[test]
    fn a_locked_file_is_reported_as_skipped() {
        let dir = scratch();
        let root = dir.path().join("root");
        fs::create_dir_all(&root).expect("dirs");
        let locked = root.join("busy.tmp");
        write_file(&locked, 42);

        // An open handle that does not share deletion blocks removal on
        // Windows, which is exactly what an in-use temporary file looks like.
        #[cfg(windows)]
        let handle = {
            use std::os::windows::fs::OpenOptionsExt;

            fs::OpenOptions::new()
                .read(true)
                .share_mode(0x1) // FILE_SHARE_READ only
                .open(&locked)
                .expect("open")
        };
        #[cfg(not(windows))]
        let handle = fs::File::open(&locked).expect("open");

        let report = clean_roots(&[root.clone()], true);
        drop(handle);

        #[cfg(windows)]
        {
            assert_eq!(report.skipped, 1);
            assert_eq!(report.files, 0);
            assert!(locked.is_file());
        }
        #[cfg(not(windows))]
        {
            assert_eq!(report.files + report.skipped, 1);
        }
    }

    #[test]
    fn missing_roots_are_ignored() {
        let dir = scratch();
        let report = clean_roots(&[dir.path().join("does-not-exist")], true);
        assert_eq!(report, CleanReport::default());
    }

    #[test]
    fn shared_folders_only_expose_their_own_files() {
        assert!(belongs(CleanKind::Cache, "thumbcache_256.db"));
        assert!(belongs(CleanKind::Cache, "iconcache_48.db"));
        assert!(!belongs(CleanKind::Cache, "settings.json"));
        assert!(!belongs(CleanKind::Temp, "thumbcache_256.db"));
    }

    /// Test entry point: same logic as [`clean`] with explicit roots.
    fn clean_roots(roots: &[PathBuf], delete: bool) -> CleanReport {
        let mut report = CleanReport::default();
        let mut budget = MAX_ENTRIES;
        for root in roots {
            if !root.is_dir() {
                continue;
            }
            report.roots.push(root.display().to_string());
            visit(root, 0, delete, &mut report, &mut budget);
        }
        report
    }
}
