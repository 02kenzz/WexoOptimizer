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
    /// What the overlay shows and where it sits. Missing fields fall back to
    /// the defaults, so settings files written by older versions still load.
    #[serde(default)]
    pub overlay: OverlaySettings,
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
            overlay_enabled: true,
            overlay: OverlaySettings::default(),
            data_dir: None,
        }
    }
}

impl AppSettings {
    /// Keeps user-supplied values inside the ranges the monitor can actually honor.
    pub fn sanitize(mut self) -> Self {
        self.monitor_interval_ms = self.monitor_interval_ms.clamp(500, 10_000);
        self.overlay = self.overlay.sanitize();
        self
    }
}

/// Which measurements the floating overlay shows, where it sits and how
/// opaque it is. Every metric is on by default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct OverlaySettings {
    pub show_fps: bool,
    pub show_cpu: bool,
    pub show_gpu: bool,
    pub show_ram: bool,
    pub show_network: bool,
    pub position: OverlayPosition,
    /// Whether the blocks stack in a column or sit side by side.
    pub orientation: OverlayOrientation,
    /// Panel opacity, 0.35-1.0.
    pub opacity: f32,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            show_fps: true,
            show_cpu: true,
            show_gpu: true,
            show_ram: true,
            show_network: true,
            position: OverlayPosition::TopRight,
            orientation: OverlayOrientation::Horizontal,
            opacity: 0.92,
        }
    }
}

impl OverlaySettings {
    /// Keeps the opacity readable: below 35% the panel stops being usable.
    pub fn sanitize(mut self) -> Self {
        self.opacity = self.opacity.clamp(0.35, 1.0);
        self
    }
}

/// Corner of the primary monitor the panel is anchored to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum OverlayPosition {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Direction the overlay arranges its blocks in: a tall column or a wide row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum OverlayOrientation {
    Vertical,
    #[default]
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum Language {
    Es,
    En,
    /// Simplified Chinese.
    Zh,
    De,
    Fr,
    Ar,
    Pl,
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
    fn language_serializes_lowercase_codes() {
        for (language, code) in [
            (Language::Es, "\"es\""),
            (Language::En, "\"en\""),
            (Language::Zh, "\"zh\""),
            (Language::De, "\"de\""),
            (Language::Fr, "\"fr\""),
            (Language::Ar, "\"ar\""),
            (Language::Pl, "\"pl\""),
        ] {
            assert_eq!(serde_json::to_string(&language).unwrap(), code);
        }
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

    #[test]
    fn overlay_defaults_show_every_metric_top_right() {
        let overlay = OverlaySettings::default();
        assert!(overlay.show_fps);
        assert!(overlay.show_cpu);
        assert!(overlay.show_gpu);
        assert!(overlay.show_ram);
        assert!(overlay.show_network);
        assert_eq!(overlay.position, OverlayPosition::TopRight);
        assert_eq!(overlay.orientation, OverlayOrientation::Horizontal);
        assert!((overlay.opacity - 0.92).abs() < f32::EPSILON);
    }

    #[test]
    fn overlay_fills_missing_fields_from_defaults() {
        let overlay: OverlaySettings =
            serde_json::from_str("{\"showNetwork\":false}").expect("parse");
        assert!(!overlay.show_network);
        assert!(overlay.show_fps);
        assert_eq!(overlay.position, OverlayPosition::TopRight);
        assert_eq!(overlay.orientation, OverlayOrientation::Horizontal);
    }

    #[test]
    fn overlay_orientation_serializes_lowercase() {
        let json = serde_json::to_string(&OverlayOrientation::Horizontal).unwrap();
        assert_eq!(json, "\"horizontal\"");
    }

    #[test]
    fn overlay_position_serializes_kebab_case() {
        let json = serde_json::to_string(&OverlayPosition::BottomLeft).unwrap();
        assert_eq!(json, "\"bottom-left\"");
    }

    #[test]
    fn sanitize_clamps_overlay_opacity() {
        let faint = AppSettings {
            overlay: OverlaySettings {
                opacity: 0.05,
                ..OverlaySettings::default()
            },
            ..AppSettings::default()
        };
        assert_eq!(faint.sanitize().overlay.opacity, 0.35);

        let opaque = AppSettings {
            overlay: OverlaySettings {
                opacity: 4.0,
                ..OverlaySettings::default()
            },
            ..AppSettings::default()
        };
        assert_eq!(opaque.sanitize().overlay.opacity, 1.0);
    }
}
