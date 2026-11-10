//! Frames per second of the foreground process, counted from ETW `Present`
//! events (`Microsoft-Windows-DXGI`, `Microsoft-Windows-D3D9`,
//! `Microsoft-Windows-DxgKrnl`) — the approach Intel PresentMon uses, with no
//! injection. Starting a real-time session needs administrator rights (or the
//! *Performance Log Users* group); without them the meter reports no value.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const FRAME_WINDOW: i64 = 20_000_000; // 2 s, in 100 ns units
const MIN_SPAN: i64 = 2_500_000; // 250 ms
/// A queue that stopped receiving presents reports nothing, instead of a stale
/// average from the last time the game drew a frame.
///
/// ETW hands real-time events over in batches, flushed once per second. While a
/// game draws every 16 ms the newest delivered event can still be a full second
/// old in wall-clock terms, so a shorter window made the meter alternate
/// between a value and nothing on every poll.
const STALE_AFTER: Duration = Duration::from_secs(2);
/// Processes that stopped presenting are dropped from the map after this long.
const IDLE_AFTER: Duration = Duration::from_secs(60);
const SESSION_NAME: &str = "WexoOptimizerFps";

/// One queue per event kind. The same frame is reported by several providers at
/// once (DXGI and the kernel), so each kind keeps its own history: mixing them
/// would count the same present twice.
const SOURCES: usize = 5;

#[derive(Default)]
struct SourceFrames {
    queues: [VecDeque<i64>; SOURCES],
    /// Wall clock of the newest event per queue.
    last: [Option<Instant>; SOURCES],
}

impl SourceFrames {
    fn push(&mut self, source: usize, timestamp: i64, now: Instant) {
        let queue = &mut self.queues[source];
        queue.push_back(timestamp);
        let cutoff = timestamp - FRAME_WINDOW;
        while queue.front().is_some_and(|value| *value < cutoff) {
            queue.pop_front();
        }
        self.last[source] = Some(now);
    }

    /// Busiest queue wins: OpenGL and Vulkan never pass through DXGI/D3D9, while
    /// D3D games are also seen by the kernel.
    fn rate(&self) -> Option<f32> {
        self.queues
            .iter()
            .zip(self.last.iter())
            .filter(|(_, last)| last.is_some_and(|instant| instant.elapsed() <= STALE_AFTER))
            .filter_map(|(queue, _)| rate_of(queue))
            .reduce(f32::max)
    }

    /// True while this process is still presenting frames.
    fn is_active(&self) -> bool {
        self.last
            .iter()
            .flatten()
            .any(|instant| instant.elapsed() <= IDLE_AFTER)
    }
}

fn rate_of(queue: &VecDeque<i64>) -> Option<f32> {
    if queue.len() < 2 {
        return None;
    }
    let span = queue.back()? - queue.front()?;
    if span < MIN_SPAN {
        return None;
    }
    let fps = (queue.len() - 1) as f64 * 10_000_000.0 / span as f64;
    (0.1..=2_000.0).contains(&fps).then_some(fps as f32)
}

/// Present timestamps per process, filled from the trace callback.
#[derive(Default)]
struct Shared {
    frames: Mutex<HashMap<u32, SourceFrames>>,
    /// Last process the meter logged, to keep the log quiet.
    logged_pid: AtomicU32,
    /// Whether that process had a value, so only transitions are logged.
    logged_rate: AtomicBool,
}

impl Shared {
    fn push(&self, process_id: u32, source: usize, timestamp: i64, now: Instant) {
        let mut frames = match self.frames.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        frames
            .entry(process_id)
            .or_default()
            .push(source, timestamp, now);
    }

    /// Rate of `process_id`, dropping processes that stopped presenting.
    fn measure(&self, process_id: u32) -> Option<f32> {
        let mut frames = match self.frames.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let rate = frames.get(&process_id).and_then(SourceFrames::rate);
        frames.retain(|_, source| source.is_active());
        rate
    }

    /// Logs the first measurement of a foreground process and every change in
    /// whether it has a value, which is what a bug report needs to see.
    fn log_transition(&self, process_id: u32, rate: Option<f32>) {
        let has_rate = rate.is_some();
        let changed = self.logged_pid.swap(process_id, Ordering::Relaxed) != process_id
            || self.logged_rate.swap(has_rate, Ordering::Relaxed) != has_rate;
        if changed {
            tracing::debug!(
                process_id,
                fps = ?rate,
                "ETW: FPS del proceso en primer plano"
            );
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use std::time::Instant;

    use tracing::{debug, warn};
    use windows_sys::core::GUID;
    use windows_sys::Win32::System::Diagnostics::Etw::{
        CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW,
        CONTROLTRACE_HANDLE, EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD,
        EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES,
        EVENT_TRACE_REAL_TIME_MODE, PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_REAL_TIME,
        TRACE_LEVEL_VERBOSE, WNODE_FLAG_TRACED_GUID,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    use super::{Shared, SESSION_NAME};

    const DXGI: GUID = GUID::from_u128(0xca11c036_0102_4a2d_a6ad_f03cfed5d3c9);
    const D3D9: GUID = GUID::from_u128(0x783aca0a_790e_4d7f_8451_aa850511c6b9);
    const KERNEL: GUID = GUID::from_u128(0x802ec45a_1e99_4b83_9920_87c98277ba9d);

    const DXGI_PRESENT_START: u16 = 0x002a;
    const DXGI_PRESENT_MPO_START: u16 = 0x0037;
    const D3D9_PRESENT_START: u16 = 0x0001;
    const KERNEL_PRESENT_HISTORY_START: u16 = 0x00ab;
    const KERNEL_PRESENT_HISTORY_DETAILED_START: u16 = 0x00d7;
    const ERROR_ALREADY_EXISTS: u32 = 183;

    fn same_guid(left: GUID, right: GUID) -> bool {
        left.data1 == right.data1
            && left.data2 == right.data2
            && left.data3 == right.data3
            && left.data4 == right.data4
    }

    fn null_handle() -> CONTROLTRACE_HANDLE {
        CONTROLTRACE_HANDLE { Value: 0 }
    }

    pub struct FpsMeter {
        shared: Arc<Shared>,
        session: Vec<u16>,
        thread: Option<JoinHandle<()>>,
    }

    impl FpsMeter {
        pub fn start() -> Option<Self> {
            let session: Vec<u16> = SESSION_NAME
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let handle = unsafe { start_session(&session)? };
            unsafe {
                enable_provider(handle, &DXGI);
                enable_provider(handle, &D3D9);
                enable_provider(handle, &KERNEL);
            }

            let shared = Arc::new(Shared::default());
            let thread_shared = Arc::clone(&shared);
            let thread_session = session.clone();
            let thread = thread::Builder::new()
                .name("wexo-fps".to_string())
                .spawn(move || unsafe { trace_loop(&thread_session, thread_shared) })
                .ok()?;

            tracing::info!("ETW: sesión de FPS iniciada");
            Some(Self {
                shared,
                session,
                thread: Some(thread),
            })
        }

        pub fn foreground_fps(&self) -> Option<f32> {
            let process_id = unsafe { foreground_process_id()? };
            if process_id == std::process::id() {
                return None;
            }
            let rate = self.shared.measure(process_id);
            self.shared.log_transition(process_id, rate);
            rate
        }
    }

    impl Drop for FpsMeter {
        fn drop(&mut self) {
            unsafe { stop_session(&self.session) };
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    unsafe fn stop_session(name: &[u16]) {
        let mut buffer = properties(name);
        ControlTraceW(
            null_handle(),
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
            EVENT_TRACE_CONTROL_STOP,
        );
    }

    /// `EVENT_TRACE_PROPERTIES` and the logger name in one aligned block.
    unsafe fn properties(name: &[u16]) -> Vec<u64> {
        let header = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
        let words = (header + name.len() * 2 + 7) / 8 + 1;
        let mut buffer = vec![0u64; words];

        let props = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        (*props).Wnode.BufferSize = (words * 8) as u32;
        (*props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        (*props).Wnode.ClientContext = 1;
        (*props).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        (*props).LoggerNameOffset = header as u32;
        (*props).LogFileNameOffset = 0;

        let destination = (buffer.as_mut_ptr() as *mut u8).add(header) as *mut u16;
        ptr::copy_nonoverlapping(name.as_ptr(), destination, name.len());
        buffer
    }

    unsafe fn start_session(name: &[u16]) -> Option<CONTROLTRACE_HANDLE> {
        let mut handle = null_handle();
        let mut buffer = properties(name);
        let status = StartTraceW(
            &mut handle,
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
        );

        if status == ERROR_ALREADY_EXISTS {
            stop_session(name);
            buffer = properties(name);
            let retry = StartTraceW(
                &mut handle,
                name.as_ptr(),
                buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
            );
            if retry != 0 {
                warn!(retry, "ETW: no se pudo reclamar la sesión de FPS");
                return None;
            }
            return Some(handle);
        }
        if status != 0 {
            warn!(status, "ETW: no se pudo iniciar la sesión de FPS");
            return None;
        }
        Some(handle)
    }

    unsafe fn enable_provider(handle: CONTROLTRACE_HANDLE, provider: &GUID) {
        let status = EnableTraceEx2(
            handle,
            provider,
            EVENT_CONTROL_CODE_ENABLE_PROVIDER,
            TRACE_LEVEL_VERBOSE as u8,
            u64::MAX,
            0,
            0,
            ptr::null(),
        );
        if status != 0 {
            debug!(status, "ETW: proveedor de present no disponible");
        }
    }

    unsafe extern "system" fn on_event(record: *mut EVENT_RECORD) {
        if record.is_null() {
            return;
        }
        let event = &*record;
        let Some(shared) = (event.UserContext as *const Shared).as_ref() else {
            return;
        };

        let header = &event.EventHeader;
        let event_id = header.EventDescriptor.Id;
        let source = if same_guid(header.ProviderId, DXGI) {
            match event_id {
                DXGI_PRESENT_START => 0,
                DXGI_PRESENT_MPO_START => 1,
                _ => return,
            }
        } else if same_guid(header.ProviderId, D3D9) && event_id == D3D9_PRESENT_START {
            2
        } else if same_guid(header.ProviderId, KERNEL) {
            match event_id {
                KERNEL_PRESENT_HISTORY_START => 3,
                KERNEL_PRESENT_HISTORY_DETAILED_START => 4,
                _ => return,
            }
        } else {
            return;
        };

        // The payload does not start with the process id (DXGI and D3D9 begin
        // with the swap-chain pointer), so read the thread that presented from
        // the header, which is what PresentMon does.
        let process_id = header.ProcessId;
        if process_id == 0 || process_id == std::process::id() {
            return;
        }
        shared.push(process_id, source, header.TimeStamp, Instant::now());
    }

    unsafe fn trace_loop(name: &[u16], shared: Arc<Shared>) {
        let mut logfile: EVENT_TRACE_LOGFILEW = std::mem::zeroed();
        logfile.LoggerName = name.as_ptr() as *mut u16;
        logfile.Anonymous1.ProcessTraceMode =
            PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
        logfile.Anonymous2.EventRecordCallback = Some(on_event);
        logfile.Context = Arc::as_ptr(&shared) as *mut c_void;

        let trace = OpenTraceW(&mut logfile);
        let status = ProcessTrace(&trace, 1, ptr::null(), ptr::null());
        if status != 0 {
            debug!(status, "ETW: ProcessTrace terminó con error");
        }
        CloseTrace(trace);
    }

    unsafe fn foreground_process_id() -> Option<u32> {
        let window = GetForegroundWindow();
        if window.is_null() {
            return None;
        }
        let mut process_id = 0u32;
        GetWindowThreadProcessId(window, &mut process_id);
        (process_id != 0).then_some(process_id)
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct FpsMeter;

    impl FpsMeter {
        pub fn start() -> Option<Self> {
            None
        }

        pub fn foreground_fps(&self) -> Option<f32> {
            None
        }
    }
}

pub use imp::FpsMeter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_needs_two_frames_and_a_span() {
        let now = Instant::now();
        let mut frames = SourceFrames::default();
        frames.push(0, 1_000, now);
        assert_eq!(frames.rate(), None);

        let mut frames = SourceFrames::default();
        for index in 0..60 {
            frames.push(0, index * 10_000_000 / 60, now);
        }
        let fps = frames.rate().expect("rate");
        assert!((fps - 60.0).abs() < 1.5, "fps = {fps}");
    }

    #[test]
    fn frames_outside_the_window_are_dropped() {
        let now = Instant::now();
        let mut frames = SourceFrames::default();
        frames.push(0, 0, now);
        frames.push(0, FRAME_WINDOW + 10_000_000, now);
        assert_eq!(frames.queues[0].len(), 1);
    }

    #[test]
    fn busiest_source_wins() {
        let now = Instant::now();
        let mut frames = SourceFrames::default();
        for index in 0..120 {
            frames.push(2, index * 10_000_000 / 120, now);
        }
        for index in 0..30 {
            frames.push(0, index * 10_000_000 / 30, now);
        }
        let fps = frames.rate().expect("rate");
        assert!((fps - 120.0).abs() < 3.0, "fps = {fps}");
    }

    #[test]
    fn each_source_is_counted_once() {
        let now = Instant::now();
        let mut frames = SourceFrames::default();
        // The same 60 frames reported by DXGI and the kernel must not add up.
        for index in 0..60 {
            frames.push(0, index * 10_000_000 / 60, now);
            frames.push(3, index * 10_000_000 / 60, now);
        }
        let fps = frames.rate().expect("rate");
        assert!((fps - 60.0).abs() < 1.5, "fps = {fps}");
    }

    #[test]
    fn a_source_that_stopped_presenting_reports_nothing() {
        let stale = Instant::now() - Duration::from_secs(5);
        let mut frames = SourceFrames::default();
        for index in 0..60 {
            frames.push(0, index * 10_000_000 / 60, stale);
        }
        assert_eq!(frames.rate(), None);
        assert!(frames.is_active());

        let forgotten = Instant::now() - Duration::from_secs(120);
        let mut frames = SourceFrames::default();
        frames.push(0, 0, forgotten);
        assert!(!frames.is_active());
    }

    #[test]
    fn a_value_survives_the_etw_flush_delay() {
        // ETW flushes once per second: between two flushes the newest event is
        // almost a second old, and the meter must still report the frames it has.
        let delivered = Instant::now() - Duration::from_millis(1_100);
        let mut frames = SourceFrames::default();
        for index in 0..60 {
            frames.push(0, index * 10_000_000 / 60, delivered);
        }
        let fps = frames.rate().expect("rate");
        assert!((fps - 60.0).abs() < 1.5, "fps = {fps}");
    }
}
