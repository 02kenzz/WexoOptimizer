use std::fs;
use std::io;
use std::path::PathBuf;

use wexo_core::AppSettings;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no se pudo acceder al archivo de configuración: {0}")]
    Io(#[from] io::Error),
    #[error("configuración inválida: {0}")]
    Json(#[from] serde_json::Error),
}

/// Well-known locations used by the app. Everything lives under one per-user
/// directory (`%LOCALAPPDATA%` on Windows, `$XDG_DATA_HOME` elsewhere), so
/// uninstalling is a simple directory removal.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub base_dir: PathBuf,
    pub settings_file: PathBuf,
    pub logs_dir: PathBuf,
    pub snapshots_dir: PathBuf,
    pub rules_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Self {
        let base_dir = data_dir().join("WexoOptimizer");

        Self {
            settings_file: base_dir.join("settings.json"),
            logs_dir: base_dir.join("logs"),
            snapshots_dir: base_dir.join("snapshots"),
            rules_dir: base_dir.join("rules"),
            base_dir,
        }
    }

    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(&self.base_dir)?;
        fs::create_dir_all(&self.logs_dir)?;
        fs::create_dir_all(&self.snapshots_dir)?;
        fs::create_dir_all(&self.rules_dir)?;
        Ok(())
    }
}

/// Per-user data directory for the current platform.
#[cfg(windows)]
fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// Per-user data directory following the XDG base-directory spec.
#[cfg(not(windows))]
fn data_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/share"))
        .unwrap_or_else(std::env::temp_dir)
}

/// Loads settings, falling back to safe defaults.
///
/// A corrupt file is preserved next to the original (`.corrupt-<timestamp>`)
/// instead of being silently destroyed.
pub fn load_settings(paths: &AppPaths) -> AppSettings {
    let raw = match fs::read_to_string(&paths.settings_file) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return AppSettings::default(),
        Err(error) => {
            tracing::warn!(%error, "no se pudo leer settings.json, usando valores por defecto");
            return AppSettings::default();
        }
    };

    match serde_json::from_str::<AppSettings>(&raw) {
        Ok(settings) => settings.sanitize(),
        Err(error) => {
            tracing::warn!(%error, "settings.json corrupto, se conserva una copia");
            let backup = paths
                .base_dir
                .join(format!("settings.corrupt-{}.json", now_ms()));
            if let Err(backup_error) = fs::write(&backup, raw) {
                tracing::warn!(%backup_error, "no se pudo guardar la copia de settings.json");
            }
            AppSettings::default()
        }
    }
}

/// Writes settings atomically (temp file in the same directory + rename).
pub fn save_settings(paths: &AppPaths, settings: &AppSettings) -> Result<(), ConfigError> {
    fs::create_dir_all(&paths.base_dir)?;

    let mut temp = tempfile::NamedTempFile::new_in(&paths.base_dir)?;
    serde_json::to_writer_pretty(&mut temp, settings)?;
    temp.as_file_mut().sync_all()?;
    temp.persist(&paths.settings_file)
        .map_err(|error| ConfigError::Io(error.error))?;

    Ok(())
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wexo_core::{Language, Theme};

    fn temp_paths() -> (tempfile::TempDir, AppPaths) {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("WexoOptimizer");
        let paths = AppPaths {
            settings_file: base.join("settings.json"),
            logs_dir: base.join("logs"),
            snapshots_dir: base.join("snapshots"),
            rules_dir: base.join("rules"),
            base_dir: base,
        };
        (dir, paths)
    }

    #[test]
    fn missing_file_returns_defaults() {
        let (_dir, paths) = temp_paths();
        let settings = load_settings(&paths);
        assert_eq!(settings.language, Language::Es);
    }

    #[test]
    fn settings_round_trip() {
        let (_dir, paths) = temp_paths();
        let settings = AppSettings {
            language: Language::En,
            theme: Theme::HighContrast,
            ..AppSettings::default()
        };
        save_settings(&paths, &settings).expect("save");

        let loaded = load_settings(&paths);
        assert_eq!(loaded.language, Language::En);
        assert_eq!(loaded.theme, Theme::HighContrast);
    }

    #[test]
    fn corrupt_file_is_backed_up_and_defaults_are_used() {
        let (_dir, paths) = temp_paths();
        std::fs::create_dir_all(&paths.base_dir).expect("dirs");
        std::fs::write(&paths.settings_file, "{ not json").expect("write");

        let settings = load_settings(&paths);
        assert_eq!(settings.language, Language::Es);

        let backups: Vec<_> = std::fs::read_dir(&paths.base_dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.corrupt-")
            })
            .collect();
        assert_eq!(backups.len(), 1);
    }
}
