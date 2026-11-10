//! Maintenance tools: disk cleanup, long-running repair commands and the data
//! behind the Tools page.
//!
//! Everything here reports what actually happened. A file that could not be
//! removed is counted as skipped, and a command that exits non-zero is a
//! failure with its exit code, never a silent success.

pub mod cleanup;
pub mod jobs;

pub use cleanup::{clean, scan, CleanKind, CleanReport};
pub use jobs::{JobKind, JobManager, JobSnapshot, JobStatus, MAX_LINES};
