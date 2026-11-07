//! Pure domain types and logic for Wexo Optimizer.
//!
//! This crate intentionally has **no Windows dependencies**: everything here must
//! stay unit-testable on any host. Hardware access lives in `wexo-sysinfo`.

pub mod app;
pub mod diagnostics;
pub mod optimization;
pub mod profile;
pub mod settings;

pub use app::AppInfo;
pub use diagnostics::{Bottleneck, BottleneckComponent, Confidence, HealthScore, Severity};
pub use optimization::{
    Applicability, BenchmarkDelta, Category, Optimization, RiskLevel, SnapshotSpec,
    TransactionRecord, TransactionStatus, VerificationResult,
};
pub use profile::{CpuInfo, DiskInfo, DiskKind, MemoryInfo, NetworkInfo, OsInfo, SystemProfile};
pub use settings::{AppSettings, Language, Theme, UpdateChannel};

/// Version of the domain model. Bump when a breaking change lands so the
/// frontend can detect stale generated bindings.
pub const MODEL_VERSION: u32 = 1;
