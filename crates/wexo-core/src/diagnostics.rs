use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Health score per subsystem plus a weighted global score (0-100).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct HealthScore {
    pub global: u8,
    pub cpu: u8,
    pub memory: u8,
    pub disk: u8,
    pub network: u8,
}

impl HealthScore {
    pub fn clamp(value: i32) -> u8 {
        value.clamp(0, 100) as u8
    }

    /// Recomputes the global score from subsystem scores using the documented weights.
    pub fn recompute_global(&mut self) {
        let weighted = f32::from(self.cpu) * 0.30
            + f32::from(self.memory) * 0.25
            + f32::from(self.disk) * 0.30
            + f32::from(self.network) * 0.15;
        self.global = Self::clamp(weighted.round() as i32);
    }
}

impl Default for HealthScore {
    fn default() -> Self {
        Self {
            global: 100,
            cpu: 100,
            memory: 100,
            disk: 100,
            network: 100,
        }
    }
}

/// A detected performance problem with supporting evidence.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct Bottleneck {
    pub component: BottleneckComponent,
    pub severity: Severity,
    pub title: String,
    pub evidence: Vec<String>,
    pub impact_estimate: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum BottleneckComponent {
    Cpu,
    Gpu,
    Memory,
    Disk,
    Network,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_score_is_clamped_and_weighted() {
        let mut score = HealthScore {
            global: 0,
            cpu: 100,
            memory: 50,
            disk: 100,
            network: 0,
        };
        score.recompute_global();
        // 30 + 12.5 + 30 + 0 = 72.5 -> 73
        assert_eq!(score.global, 73);

        assert_eq!(HealthScore::clamp(-5), 0);
        assert_eq!(HealthScore::clamp(150), 100);
    }

    #[test]
    fn severity_is_ordered() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }
}
