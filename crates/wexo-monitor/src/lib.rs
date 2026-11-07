//! Background metric sampling and in-memory history.
//!
//! M0 ships the ring buffer used by the monitor UI; the sampling service lands in M3.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetricSample {
    pub timestamp_ms: u64,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub disk_percent: f32,
    pub network_rx_bytes_per_sec: u64,
    pub network_tx_bytes_per_sec: u64,
}

/// Fixed-capacity history, oldest samples dropped first.
#[derive(Debug)]
pub struct RingBuffer {
    capacity: usize,
    samples: VecDeque<MetricSample>,
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

    pub fn push(&mut self, sample: MetricSample) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    pub fn latest(&self) -> Option<&MetricSample> {
        self.samples.back()
    }

    pub fn iter(&self) -> impl Iterator<Item = &MetricSample> {
        self.samples.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ts: u64) -> MetricSample {
        MetricSample {
            timestamp_ms: ts,
            cpu_percent: 0.0,
            memory_percent: 0.0,
            disk_percent: 0.0,
            network_rx_bytes_per_sec: 0,
            network_tx_bytes_per_sec: 0,
        }
    }

    #[test]
    fn buffer_drops_oldest_samples() {
        let mut buffer = RingBuffer::new(2);
        buffer.push(sample(1));
        buffer.push(sample(2));
        buffer.push(sample(3));
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.latest().map(|s| s.timestamp_ms), Some(3));
        let timestamps: Vec<u64> = buffer.iter().map(|s| s.timestamp_ms).collect();
        assert_eq!(timestamps, vec![2, 3]);
    }

    #[test]
    fn capacity_is_at_least_one() {
        assert_eq!(RingBuffer::new(0).capacity(), 1);
    }
}
