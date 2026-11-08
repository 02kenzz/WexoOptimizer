//! Background metric sampling and in-memory history.
//!
//! [`MonitorService`] owns a dedicated thread that periodically asks a
//! [`MetricSource`] for a sample, appends it to a bounded history and forwards
//! it to a [`SampleSink`] (the Tauri layer uses it to emit events).
//!
//! Sampling and history live here instead of in the Tauri command layer so the
//! behaviour can be tested with a fake source, without Windows or a webview.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub use wexo_core::LiveMetrics;

/// Samples kept in memory. At the default 1 s interval this is 30 minutes.
pub const DEFAULT_HISTORY_CAPACITY: usize = 1_800;
/// Slowest supported sampling interval.
pub const MAX_INTERVAL: Duration = Duration::from_secs(10);
/// Fastest supported sampling interval, to keep CPU cost negligible.
pub const MIN_INTERVAL: Duration = Duration::from_millis(500);

/// Produces one sample per tick. Implementations are owned by the monitor
/// thread, so they do not need to be `Sync`.
pub trait MetricSource: Send + 'static {
    fn sample(&mut self) -> LiveMetrics;
}

/// Receives every sample after it has been stored in the history.
///
/// Called on the monitor thread: implementations must be fast and must not
/// block (the Tauri sink just emits an event).
pub trait SampleSink: Send + Sync + 'static {
    fn on_sample(&self, sample: &LiveMetrics);
}

impl<F> SampleSink for F
where
    F: Fn(&LiveMetrics) + Send + Sync + 'static,
{
    fn on_sample(&self, sample: &LiveMetrics) {
        self(sample);
    }
}

/// Fixed-capacity history, oldest samples dropped first.
#[derive(Debug)]
pub struct RingBuffer {
    capacity: usize,
    samples: VecDeque<LiveMetrics>,
}

impl RingBuffer {
    /// `capacity` is clamped to at least one sample.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            samples: VecDeque::with_capacity(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn push(&mut self, sample: LiveMetrics) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    pub fn latest(&self) -> Option<&LiveMetrics> {
        self.samples.back()
    }

    pub fn iter(&self) -> impl Iterator<Item = &LiveMetrics> {
        self.samples.iter()
    }

    pub fn snapshot(&self) -> Vec<LiveMetrics> {
        self.samples.iter().cloned().collect()
    }
}

enum MonitorCommand {
    SetInterval(Duration),
    Stop,
}

/// Handle to a running monitor thread.
///
/// Dropping the handle stops and joins the thread, so the app can shut down
/// cleanly without leaving a detached sampler behind.
pub struct MonitorHandle {
    history: Arc<RwLock<RingBuffer>>,
    interval_ms: Arc<AtomicU64>,
    commands: Sender<MonitorCommand>,
    thread: Option<JoinHandle<()>>,
}

impl MonitorHandle {
    /// Starts sampling immediately, then every `interval`.
    ///
    /// The interval is clamped to [`MIN_INTERVAL`]..=[`MAX_INTERVAL`].
    pub fn start<S, K>(source: S, sink: K, interval: Duration, capacity: usize) -> Self
    where
        S: MetricSource,
        K: SampleSink,
    {
        let interval = clamp_interval(interval);
        let history = Arc::new(RwLock::new(RingBuffer::new(capacity)));
        let interval_ms = Arc::new(AtomicU64::new(interval.as_millis() as u64));
        let (commands, receiver) = mpsc::channel();

        let thread_history = Arc::clone(&history);
        let thread_interval = Arc::clone(&interval_ms);

        let spawned = thread::Builder::new()
            .name("wexo-monitor".to_string())
            .spawn(move || {
                run(
                    source,
                    sink,
                    receiver,
                    thread_history,
                    thread_interval,
                    interval,
                );
            });

        let thread = match spawned {
            Ok(thread) => Some(thread),
            Err(error) => {
                tracing::error!(%error, "no se pudo iniciar el hilo del monitor");
                None
            }
        };

        Self {
            history,
            interval_ms,
            commands,
            thread,
        }
    }

    /// A copy of the current history, oldest first.
    pub fn history(&self) -> Vec<LiveMetrics> {
        read_history(&self.history).snapshot()
    }

    /// Most recent sample, if the monitor has produced one yet.
    pub fn latest(&self) -> Option<LiveMetrics> {
        read_history(&self.history).latest().cloned()
    }

    /// Current sampling interval in milliseconds, as applied by the thread.
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms.load(Ordering::Acquire)
    }

    /// Requests a new interval. Returns `false` if the thread is no longer
    /// running.
    pub fn set_interval(&self, interval: Duration) -> bool {
        let interval = clamp_interval(interval);
        self.commands
            .send(MonitorCommand::SetInterval(interval))
            .is_ok()
    }

    /// Stops the thread and waits for it to finish. Idempotent.
    pub fn stop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = self.commands.send(MonitorCommand::Stop);
            if thread.join().is_err() {
                tracing::warn!("el hilo del monitor terminó de forma anómala");
            }
        }
    }
}

impl Drop for MonitorHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn clamp_interval(interval: Duration) -> Duration {
    interval.clamp(MIN_INTERVAL, MAX_INTERVAL)
}

fn read_history(history: &RwLock<RingBuffer>) -> std::sync::RwLockReadGuard<'_, RingBuffer> {
    match history.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn run<S, K>(
    mut source: S,
    sink: K,
    receiver: mpsc::Receiver<MonitorCommand>,
    history: Arc<RwLock<RingBuffer>>,
    interval_ms: Arc<AtomicU64>,
    initial_interval: Duration,
) where
    S: MetricSource,
    K: SampleSink,
{
    let mut interval = initial_interval;
    let mut next_tick = Instant::now() + interval;

    loop {
        match receiver.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
            Ok(MonitorCommand::SetInterval(new_interval)) => {
                interval = new_interval;
                interval_ms.store(interval.as_millis() as u64, Ordering::Release);
                // Restart the cadence so the change takes effect right away
                // without firing a burst of catch-up samples.
                next_tick = Instant::now() + interval;
            }
            Ok(MonitorCommand::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                let sample = source.sample();

                match history.write() {
                    Ok(mut guard) => guard.push(sample.clone()),
                    Err(poisoned) => poisoned.into_inner().push(sample.clone()),
                }
                sink.on_sample(&sample);

                next_tick += interval;
                // The machine slept or a sample took too long: skip missed
                // ticks instead of running them back-to-back.
                if next_tick <= Instant::now() {
                    next_tick = Instant::now() + interval;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct CountingSource {
        count: Arc<AtomicU64>,
    }

    impl MetricSource for CountingSource {
        fn sample(&mut self) -> LiveMetrics {
            let index = self.count.fetch_add(1, Ordering::SeqCst) + 1;
            LiveMetrics::empty(index * 1_000, index)
        }
    }

    struct CollectingSink {
        samples: Arc<Mutex<Vec<LiveMetrics>>>,
    }

    impl SampleSink for CollectingSink {
        fn on_sample(&self, sample: &LiveMetrics) {
            self.samples
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(sample.clone());
        }
    }

    fn wait_until(mut predicate: impl FnMut() -> bool, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if predicate() {
                return true;
            }
            thread::sleep(Duration::from_millis(10));
        }
        predicate()
    }

    #[test]
    fn buffer_drops_oldest_samples() {
        let mut buffer = RingBuffer::new(2);
        buffer.push(LiveMetrics::empty(1, 0));
        buffer.push(LiveMetrics::empty(2, 0));
        buffer.push(LiveMetrics::empty(3, 0));
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.latest().map(|s| s.timestamp_ms), Some(3));
        let timestamps: Vec<u64> = buffer.iter().map(|s| s.timestamp_ms).collect();
        assert_eq!(timestamps, vec![2, 3]);
    }

    #[test]
    fn capacity_is_at_least_one() {
        assert_eq!(RingBuffer::new(0).capacity(), 1);
    }

    #[test]
    fn service_samples_and_fills_history() {
        let count = Arc::new(AtomicU64::new(0));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let handle = MonitorHandle::start(
            CountingSource {
                count: Arc::clone(&count),
            },
            CollectingSink {
                samples: Arc::clone(&samples),
            },
            Duration::from_millis(20),
            10,
        );

        assert!(wait_until(
            || count.load(Ordering::SeqCst) >= 3,
            Duration::from_secs(2)
        ));

        let history = handle.history();
        let latest = handle.latest().expect("latest sample");
        assert!(!history.is_empty());
        assert_eq!(
            latest.timestamp_ms,
            history.last().map(|s| s.timestamp_ms).unwrap()
        );
        assert!(history.len() <= 10);
        // The history is bounded, so the sink may hold more samples than it.
        assert!(samples.lock().unwrap().len() >= history.len());
    }

    #[test]
    fn interval_changes_are_applied_and_clamped() {
        let count = Arc::new(AtomicU64::new(0));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let mut handle = MonitorHandle::start(
            CountingSource {
                count: Arc::clone(&count),
            },
            CollectingSink {
                samples: Arc::clone(&samples),
            },
            Duration::from_millis(500),
            10,
        );

        assert_eq!(handle.interval_ms(), 500);
        assert!(handle.set_interval(Duration::from_millis(20)));
        assert!(wait_until(
            || handle.interval_ms() == MIN_INTERVAL.as_millis() as u64,
            Duration::from_secs(2)
        ));

        let before = count.load(Ordering::SeqCst);
        assert!(wait_until(
            || count.load(Ordering::SeqCst) > before,
            Duration::from_secs(2)
        ));

        handle.stop();
    }

    #[test]
    fn stop_joins_the_thread_and_is_idempotent() {
        let count = Arc::new(AtomicU64::new(0));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let mut handle = MonitorHandle::start(
            CountingSource {
                count: Arc::clone(&count),
            },
            CollectingSink {
                samples: Arc::clone(&samples),
            },
            Duration::from_millis(20),
            10,
        );

        assert!(wait_until(
            || count.load(Ordering::SeqCst) >= 2,
            Duration::from_secs(2)
        ));
        handle.stop();
        let after_stop = count.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(80));
        assert_eq!(count.load(Ordering::SeqCst), after_stop);

        // Calling it again, or dropping, must not panic.
        handle.stop();
    }

    #[test]
    fn history_is_usable_after_drop() {
        let history = {
            let count = Arc::new(AtomicU64::new(0));
            let samples = Arc::new(Mutex::new(Vec::new()));
            let handle = MonitorHandle::start(
                CountingSource {
                    count: Arc::clone(&count),
                },
                CollectingSink {
                    samples: Arc::clone(&samples),
                },
                Duration::from_millis(20),
                10,
            );
            assert!(wait_until(
                || count.load(Ordering::SeqCst) >= 2,
                Duration::from_secs(2)
            ));
            handle.history()
        };

        assert!(!history.is_empty());
        assert!(history
            .windows(2)
            .all(|pair| pair[0].timestamp_ms <= pair[1].timestamp_ms));
    }
}
