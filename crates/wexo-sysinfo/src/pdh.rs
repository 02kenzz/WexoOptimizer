//! PDH performance-counter wrapper.
//!
//! Windows keeps English counter names registered even on localized installs,
//! so [`PdhAddEnglishCounterW`] is used instead of the localized
//! `PdhAddCounterW`. Every counter is optional: if one is missing (corrupted
//! registry, unusual SKU) the rest keep working and the sampler falls back to
//! `sysinfo` for the missing metric.

use tracing::warn;

/// Dynamic values read from PDH in one collection pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct PdhSample {
    pub cpu_percent: Option<f32>,
    pub disk_busy_percent: Option<f32>,
    pub disk_queue_length: Option<f32>,
    pub disk_read_bytes_per_sec: u64,
    pub disk_write_bytes_per_sec: u64,
}

#[cfg(windows)]
mod imp {
    use std::ptr;

    use tracing::{debug, warn};
    use windows_sys::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE,
    };

    type PdhQuery = isize;
    type PdhCounter = isize;

    use super::PdhSample;

    const CPU_UTILITY: &str = r"\Processor Information(_Total)\% Processor Utility";
    const CPU_TIME: &str = r"\Processor Information(_Total)\% Processor Time";
    const CPU_TIME_LEGACY: &str = r"\Processor(_Total)\% Processor Time";
    const DISK_IDLE: &str = r"\PhysicalDisk(_Total)\% Idle Time";
    const DISK_QUEUE: &str = r"\PhysicalDisk(_Total)\Avg. Disk Queue Length";
    const DISK_READ: &str = r"\PhysicalDisk(_Total)\Disk Read Bytes/sec";
    const DISK_WRITE: &str = r"\PhysicalDisk(_Total)\Disk Write Bytes/sec";

    fn wide(path: &str) -> Vec<u16> {
        path.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn add_counter(query: PdhQuery, path: &str) -> Option<PdhCounter> {
        let wide_path = wide(path);
        let mut counter: PdhCounter = 0;
        let status = PdhAddEnglishCounterW(query, wide_path.as_ptr(), 0, &mut counter);
        if status == 0 {
            Some(counter)
        } else {
            debug!(path, status, "PDH counter unavailable");
            None
        }
    }

    /// Formats one counter value. Rejects stale/invalid statuses.
    unsafe fn read(counter: Option<PdhCounter>) -> Option<f64> {
        let counter = counter?;
        let mut value: PDH_FMT_COUNTERVALUE = std::mem::zeroed();
        let status =
            PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, ptr::null_mut(), &mut value);
        if status != 0 {
            return None;
        }
        // PDH_CSTATUS_VALID_DATA (0) or PDH_CSTATUS_NEW_DATA (1).
        if value.CStatus > 1 {
            return None;
        }
        let double = value.Anonymous.doubleValue;
        if double.is_finite() {
            Some(double)
        } else {
            None
        }
    }

    pub struct PdhSampler {
        query: PdhQuery,
        cpu: Option<PdhCounter>,
        disk_idle: Option<PdhCounter>,
        disk_queue: Option<PdhCounter>,
        disk_read: Option<PdhCounter>,
        disk_write: Option<PdhCounter>,
    }

    impl PdhSampler {
        pub fn new() -> Option<Self> {
            unsafe {
                let mut query: PdhQuery = 0;
                if PdhOpenQueryW(ptr::null(), 0, &mut query) != 0 {
                    warn!("PDH: no se pudo abrir la consulta; se usará sysinfo");
                    return None;
                }

                let cpu = add_counter(query, CPU_UTILITY)
                    .or_else(|| add_counter(query, CPU_TIME))
                    .or_else(|| add_counter(query, CPU_TIME_LEGACY));

                let sampler = Self {
                    query,
                    cpu,
                    disk_idle: add_counter(query, DISK_IDLE),
                    disk_queue: add_counter(query, DISK_QUEUE),
                    disk_read: add_counter(query, DISK_READ),
                    disk_write: add_counter(query, DISK_WRITE),
                };

                // Prime rate counters; the first formatted value is only valid
                // after two collections separated by some time.
                PdhCollectQueryData(query);

                if sampler.cpu.is_none()
                    && sampler.disk_idle.is_none()
                    && sampler.disk_queue.is_none()
                {
                    warn!("PDH: ningún contador disponible");
                    return None;
                }

                Some(sampler)
            }
        }

        pub fn sample(&mut self) -> PdhSample {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return PdhSample::default();
                }

                let cpu_percent = read(self.cpu).map(|value| value.clamp(0.0, 100.0) as f32);
                let disk_busy_percent =
                    read(self.disk_idle).map(|idle| (100.0 - idle).clamp(0.0, 100.0) as f32);
                let disk_queue_length = read(self.disk_queue).map(|value| value.max(0.0) as f32);

                PdhSample {
                    cpu_percent,
                    disk_busy_percent,
                    disk_queue_length,
                    disk_read_bytes_per_sec: read(self.disk_read)
                        .map(|value| value.max(0.0) as u64)
                        .unwrap_or(0),
                    disk_write_bytes_per_sec: read(self.disk_write)
                        .map(|value| value.max(0.0) as u64)
                        .unwrap_or(0),
                }
            }
        }
    }

    impl Drop for PdhSampler {
        fn drop(&mut self) {
            unsafe {
                if self.query != 0 {
                    PdhCloseQuery(self.query);
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::PdhSample;

    pub struct PdhSampler;

    impl PdhSampler {
        pub fn new() -> Option<Self> {
            None
        }

        pub fn sample(&mut self) -> PdhSample {
            PdhSample::default()
        }
    }
}

pub use imp::PdhSampler;

/// Logs a one-time warning when construction fails; keeps the API infallible.
pub fn try_open() -> Option<PdhSampler> {
    let sampler = PdhSampler::new();
    if sampler.is_none() {
        warn!("PDH no disponible; se usarán las métricas de sysinfo");
    }
    sampler
}

#[cfg(all(test, windows))]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use super::*;

    #[test]
    fn opens_and_reads_real_counters() {
        let mut sampler = PdhSampler::new().expect("PDH counters should be available on Windows");
        sleep(Duration::from_millis(350));
        let sample = sampler.sample();

        let cpu = sample.cpu_percent.expect("cpu counter");
        assert!((0.0..=100.0).contains(&cpu), "cpu = {cpu}");
        if let Some(busy) = sample.disk_busy_percent {
            assert!((0.0..=100.0).contains(&busy), "busy = {busy}");
        }
        assert!(sample.disk_queue_length.unwrap_or(0.0) >= 0.0);
    }
}
