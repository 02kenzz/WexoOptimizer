//! GPU utilization from the Windows "GPU Engine" performance counters.
//!
//! Windows exposes one `Utilization Percentage` instance per engine and
//! process; the task manager's figure is their sum. The counter set only exists
//! on Windows 10 1709+ with a WDDM 2.x driver, so every failure path reports
//! `None` instead of a made-up zero.

#[cfg(windows)]
mod imp {
    use std::ptr;

    use tracing::debug;
    use windows_sys::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA,
    };

    const GPU_ENGINE: &str = r"\GPU Engine(*)\Utilization Percentage";

    pub struct GpuSampler {
        query: isize,
        counter: isize,
        /// `u64` so the PDH item array stays 8-byte aligned.
        buffer: Vec<u64>,
    }

    impl GpuSampler {
        pub fn new() -> Self {
            let mut sampler = Self {
                query: 0,
                counter: 0,
                buffer: Vec::new(),
            };
            unsafe {
                if PdhOpenQueryW(ptr::null(), 0, &mut sampler.query) != 0 {
                    debug!("PDH: no se pudo abrir la consulta de GPU");
                    return sampler;
                }
                let path: Vec<u16> = GPU_ENGINE
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let status =
                    PdhAddEnglishCounterW(sampler.query, path.as_ptr(), 0, &mut sampler.counter);
                if status != 0 {
                    debug!(status, "PDH: contador de GPU no disponible");
                    sampler.counter = 0;
                    return sampler;
                }
                // Rate counters need two collections before they produce values.
                PdhCollectQueryData(sampler.query);
            }
            sampler
        }

        /// Summed utilization of every GPU engine, clamped to 0-100.
        pub fn sample(&mut self) -> Option<f32> {
            if self.counter == 0 {
                return None;
            }
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }

                let mut size: u32 = 0;
                let mut count: u32 = 0;
                let status = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    ptr::null_mut(),
                );
                if status != PDH_MORE_DATA || size == 0 {
                    return None;
                }

                let words = (size as usize).div_ceil(8);
                self.buffer.clear();
                self.buffer.resize(words, 0);

                let items = self.buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                let status = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    items,
                );
                if status != 0 {
                    debug!(status, "PDH: no se pudo leer el uso de GPU");
                    return None;
                }

                let entries = std::slice::from_raw_parts(items, count as usize);
                let mut total = 0.0f64;
                for entry in entries {
                    // PDH_CSTATUS_VALID_DATA (0) or PDH_CSTATUS_NEW_DATA (1).
                    if entry.FmtValue.CStatus > 1 {
                        continue;
                    }
                    let value = entry.FmtValue.Anonymous.doubleValue;
                    if value.is_finite() {
                        total += value.max(0.0);
                    }
                }
                Some((total as f32).clamp(0.0, 100.0))
            }
        }
    }

    impl Drop for GpuSampler {
        fn drop(&mut self) {
            unsafe {
                if self.query != 0 {
                    PdhCloseQuery(self.query);
                }
            }
        }
    }

    impl Default for GpuSampler {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct GpuSampler;

    impl GpuSampler {
        pub fn new() -> Self {
            Self
        }

        pub fn sample(&mut self) -> Option<f32> {
            None
        }
    }

    impl Default for GpuSampler {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub use imp::GpuSampler;

/// Builds the sampler; a missing counter set only means GPU usage is unknown.
pub fn open() -> GpuSampler {
    let mut sampler = GpuSampler::new();
    if sampler.sample().is_none() {
        tracing::debug!("uso de GPU no disponible en este equipo");
    }
    sampler
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn reads_a_plausible_gpu_value() {
        let mut sampler = GpuSampler::new();
        std::thread::sleep(std::time::Duration::from_millis(400));
        if let Some(value) = sampler.sample() {
            assert!((0.0..=100.0).contains(&value), "gpu = {value}");
        }
    }
}
