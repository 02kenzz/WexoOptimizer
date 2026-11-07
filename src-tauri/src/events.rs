//! Names of the events emitted from Rust to the frontend.
//!
//! Keep them stable: the frontend listens by exact string.
#![allow(dead_code)]

/// Batched monitor metrics: `{ timestampMs, cpuPercent, ... }`.
pub const MONITOR_METRICS: &str = "monitor://metrics";

/// Long-running task progress: `{ taskId, step, total, label }`.
pub const TASK_PROGRESS: &str = "task://progress";

/// Emitted once when a batch of optimizations finishes.
pub const OPTIMIZATION_FINISHED: &str = "optimization://finished";
