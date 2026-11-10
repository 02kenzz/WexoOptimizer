use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Serializable description of a single optimization.
///
/// The executable half (apply/verify/rollback) lives in `wexo-optimizer`; this
/// type is what the frontend renders and what the transaction log records.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct Optimization {
    /// Stable kebab-case identifier. Never reused once shipped.
    pub id: String,
    pub category: Category,
    pub title: String,
    pub description: String,
    pub rationale: String,
    pub risk: RiskLevel,
    pub requires_reboot: bool,
    pub applies_to: Applicability,
    pub snapshot: SnapshotSpec,
    pub available: bool,
    /// True when applying the change needs an elevated process.
    pub requires_admin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum Category {
    Power,
    Services,
    Startup,
    FileSystem,
    Network,
    Memory,
    Gpu,
    Visuals,
    Privacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct Applicability {
    /// Minimum Windows build number, e.g. `"19044"` for 21H2.
    pub os_min_build: Option<String>,
    /// Free-form hardware filters evaluated by the rules engine.
    pub hardware_filters: Vec<String>,
    /// Other optimization IDs that conflict with this one.
    pub conflicts_with: Vec<String>,
}

/// Everything that must be preserved to roll an optimization back.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct SnapshotSpec {
    /// Full registry key paths, e.g. `HKLM\SYSTEM\CurrentControlSet\...`.
    pub registry_keys: Vec<String>,
    /// Windows service names whose state must be preserved.
    pub services: Vec<String>,
    /// Absolute file paths that will be backed up.
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "status")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum VerificationResult {
    Verified,
    NotApplied,
    Mismatch { details: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct BenchmarkDelta {
    pub metric: String,
    pub before: f64,
    pub after: f64,
    pub unit: String,
    pub delta_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct TransactionRecord {
    pub id: String,
    #[ts(type = "number")]
    pub timestamp_ms: u64,
    pub optimization_ids: Vec<String>,
    pub status: TransactionStatus,
    pub rollback_available: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum TransactionStatus {
    Applied,
    RolledBack,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_levels_are_ordered() {
        assert!(RiskLevel::High > RiskLevel::Medium);
        assert!(RiskLevel::Medium > RiskLevel::Low);
    }

    #[test]
    fn verification_uses_internal_tag() {
        let json = serde_json::to_string(&VerificationResult::Mismatch {
            details: "service still running".into(),
        })
        .unwrap();
        assert!(json.contains("\"status\":\"mismatch\""));
    }
}
