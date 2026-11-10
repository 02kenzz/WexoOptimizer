use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// User-facing application settings, persisted by the Tauri shell.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct AppSettings {
    pub language: Language,
    pub theme: Theme,
    /// Sampling interval for the monitor, in milliseconds.
    pub monitor_interval_ms: u32,
    pub minimize_to_tray: bool,
    pub update_channel: UpdateChannel,
    /// Opt-in telemetry. Off by default, always.
    pub telemetry_enabled: bool,
    /// Shows the always-on-top performance overlay window.
    #[serde(default)]
    pub overlay_enabled: bool,
    /// Custom data directory for snapshots and transaction log. None = use default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Es,
            theme: Theme::System,
            monitor_interval_ms: 1000,
            minimize_to_tray: true,
            update_channel: UpdateChannel::Stable,
            telemetry_enabled: false,
            overlay_enabled: false,
            data_dir: None,
        }
    }
}

impl AppSettings {
    /// Keeps user-supplied values inside the ranges the monitor can actually honor.
    pub fn sanitize(mut self) -> Self {
        self.monitor_interval_ms = self.monitor_interval_ms.clamp(500, 10_000);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum Language {
    Es,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum Theme {
    System,
    Light,
    Dark,
    HighContrast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum UpdateChannel {
    Stable,
    Beta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe_and_spanish() {
        let settings = AppSettings::default();
        assert_eq!(settings.language, Language::Es);
        assert!(!settings.telemetry_enabled);
        assert_eq!(settings.monitor_interval_ms, 1000);
    }

    #[test]
    fn sanitize_clamps_interval() {
        let low = AppSettings {
            monitor_interval_ms: 10,
            ..AppSettings::default()
        };
        assert_eq!(low.sanitize().monitor_interval_ms, 500);

        let high = AppSettings {
            monitor_interval_ms: 50_000,
            ..AppSettings::default()
        };
        assert_eq!(high.sanitize().monitor_interval_ms, 10_000);
    }

    #[test]
    fn theme_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Theme::HighContrast).unwrap(),
            "\"high-contrast\""
        );
    }
}
