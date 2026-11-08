//! Snapshot capture and restore for atomic optimization rollback.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json;
use thiserror::Error;
use tracing::{debug};
use winreg::enums::*;
use winreg::{RegKey, RegValue};

/// Errors that can occur during snapshot operations.
#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Registry error: {0}")]
    Registry(std::io::Error),
    #[error("Service error: {0}")]
    Service(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Missing snapshot data: {0}")]
    MissingData(String),
}

/// A captured snapshot of system state before an optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub timestamp_ms: u64,
    pub optimization_ids: Vec<String>,
    pub registry_values: HashMap<String, RegistryValue>,
    pub service_states: HashMap<String, ServiceState>,
    pub file_backups: HashMap<String, Vec<u8>>,
}

/// A single registry value captured for rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryValue {
    pub key_path: String,
    pub value_name: String,
    pub value_type: u32,
    pub data: Vec<u8>,
}

/// A Windows service state captured for rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    pub name: String,
    pub start_type: u32,
    pub status: u32,
    pub description: Option<String>,
}

/// Captures a snapshot based on the provided SnapshotSpec.
pub fn capture_snapshot(
    snapshot_dir: &Path,
    spec: &wexo_core::SnapshotSpec,
    optimization_ids: &[String],
) -> Result<Snapshot, SnapshotError> {
    let id = format!("snap-{}", now_ms());
    let timestamp_ms = now_ms();

    let mut registry_values = HashMap::new();
    let mut service_states = HashMap::new();
    let mut file_backups = HashMap::new();

    // Capture registry keys
    for key_path in &spec.registry_keys {
        capture_registry_key(&mut registry_values, key_path)?;
    }

    // Capture service states
    for service_name in &spec.services {
        let state = capture_service_state(service_name)?;
        service_states.insert(service_name.clone(), state);
    }

    // Backup files
    for file_path in &spec.files {
        let path = Path::new(file_path);
        if path.exists() {
            let data = fs::read(path)?;
            file_backups.insert(file_path.clone(), data);
        }
    }

    let snapshot = Snapshot {
        id: id.clone(),
        timestamp_ms,
        optimization_ids: optimization_ids.to_vec(),
        registry_values,
        service_states,
        file_backups,
    };

    // Persist snapshot to disk
    let snapshot_path = snapshot_dir.join(format!("{}.json", id));
    fs::create_dir_all(snapshot_dir)?;
    let json = serde_json::to_vec_pretty(&snapshot)?;
    fs::write(&snapshot_path, json)?;

    debug!(snapshot_id = %id, "snapshot captured");
    Ok(snapshot)
}

/// Restores a snapshot, rolling back all captured state.
pub fn restore_snapshot(snapshot: &Snapshot) -> Result<(), SnapshotError> {
    // Restore registry values
    for (_key, value) in &snapshot.registry_values {
        restore_registry_value(value)?;
    }

    // Restore service states
    for (name, state) in &snapshot.service_states {
        restore_service_state(name, state)?;
    }

    // Restore files
    for (path, data) in &snapshot.file_backups {
        fs::write(path, data)?;
    }

    debug!(snapshot_id = %snapshot.id, "snapshot restored");
    Ok(())
}

/// Loads a snapshot from disk.
pub fn load_snapshot(snapshot_dir: &Path, id: &str) -> Result<Snapshot, SnapshotError> {
    let path = snapshot_dir.join(format!("{}.json", id));
    let json = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

/// Lists all available snapshots.
pub fn list_snapshots(snapshot_dir: &Path) -> Result<Vec<Snapshot>, SnapshotError> {
    if !snapshot_dir.exists() {
        return Ok(Vec::new());
    }
    let mut snapshots = Vec::new();
    for entry in fs::read_dir(snapshot_dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(snapshot) = load_snapshot(snapshot_dir, &entry.file_name().to_string_lossy().trim_end_matches(".json")) {
                snapshots.push(snapshot);
            }
        }
    }
    snapshots.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
    Ok(snapshots)
}

fn capture_registry_key(
    values: &mut HashMap<String, RegistryValue>,
    key_path: &str,
) -> Result<(), SnapshotError> {
    let (root_key, sub_key) = parse_registry_path(key_path)?;
    let hklm = RegKey::predef(root_key);
    let key = hklm.open_subkey_with_flags(sub_key, KEY_READ)?;

    for value in key.enum_values().map(|v| v.unwrap()) {
        let (value_type, data) = match value.1.vtype {
            winreg::enums::RegType::REG_SZ => (winreg::enums::RegType::REG_SZ as u32, value.1.bytes),
            winreg::enums::RegType::REG_EXPAND_SZ => (winreg::enums::RegType::REG_EXPAND_SZ as u32, value.1.bytes),
            winreg::enums::RegType::REG_MULTI_SZ => (winreg::enums::RegType::REG_MULTI_SZ as u32, value.1.bytes),
            winreg::enums::RegType::REG_DWORD => (winreg::enums::RegType::REG_DWORD as u32, value.1.bytes),
            winreg::enums::RegType::REG_QWORD => (winreg::enums::RegType::REG_QWORD as u32, value.1.bytes),
            winreg::enums::RegType::REG_BINARY => (winreg::enums::RegType::REG_BINARY as u32, value.1.bytes),
            t => (t as u32, value.1.bytes),
        };

        let full_key = format!("{}\\{}", key_path, value.0);
        values.insert(
            full_key.clone(),
            RegistryValue {
                key_path: key_path.to_string(),
                value_name: value.0,
                value_type,
                data,
            },
        );
    }

    // Also capture subkeys recursively (but limit depth to avoid huge snapshots)
    capture_subkeys(values, &key, key_path, 0, 3)?;

    Ok(())
}

fn capture_subkeys(
    values: &mut HashMap<String, RegistryValue>,
    parent_key: &RegKey,
    parent_path: &str,
    depth: u32,
    max_depth: u32,
) -> Result<(), SnapshotError> {
    if depth >= max_depth {
        return Ok(());
    }

    for subkey_name in parent_key.enum_keys().map(|k| k.unwrap()) {
        let subkey_path = format!("{}\\{}", parent_path, subkey_name);
        if let Ok(subkey) = parent_key.open_subkey_with_flags(&subkey_name, KEY_READ) {
            capture_registry_key(values, &subkey_path)?;
            capture_subkeys(values, &subkey, &subkey_path, depth + 1, max_depth)?;
        }
    }
    Ok(())
}

fn restore_registry_value(value: &RegistryValue) -> Result<(), SnapshotError> {
    let (root_key, sub_key) = parse_registry_path(&value.key_path)?;
    let hklm = RegKey::predef(root_key);
    let key = hklm.open_subkey_with_flags(sub_key, KEY_WRITE)?;

    let reg_value = match value.value_type {
        t if t == winreg::enums::RegType::REG_SZ as u32 => RegValue {
            bytes: value.data.clone(),
            vtype: winreg::enums::RegType::REG_SZ,
        },
        t if t == winreg::enums::RegType::REG_EXPAND_SZ as u32 => RegValue {
            bytes: value.data.clone(),
            vtype: winreg::enums::RegType::REG_EXPAND_SZ,
        },
        t if t == winreg::enums::RegType::REG_MULTI_SZ as u32 => RegValue {
            bytes: value.data.clone(),
            vtype: winreg::enums::RegType::REG_MULTI_SZ,
        },
        t if t == winreg::enums::RegType::REG_DWORD as u32 => RegValue {
            bytes: value.data.clone(),
            vtype: winreg::enums::RegType::REG_DWORD,
        },
        t if t == winreg::enums::RegType::REG_QWORD as u32 => RegValue {
            bytes: value.data.clone(),
            vtype: winreg::enums::RegType::REG_QWORD,
        },
        t if t == winreg::enums::RegType::REG_BINARY as u32 => RegValue {
            bytes: value.data.clone(),
            vtype: winreg::enums::RegType::REG_BINARY,
        },
        _ => RegValue {
            bytes: value.data.clone(),
            vtype: winreg::enums::RegType::REG_BINARY,
        },
    };

    key.set_raw_value(&value.value_name, &reg_value)?;
    Ok(())
}

fn capture_service_state(name: &str) -> Result<ServiceState, SnapshotError> {
    use std::process::Command;

    // Get service config via sc.exe
    let config_output = Command::new("sc.exe")
        .args(["qc", name])
        .output()
        .map_err(|e| SnapshotError::Service(e.to_string()))?;

    let config = String::from_utf8_lossy(&config_output.stdout);
    let start_type = parse_sc_start_type(&config).unwrap_or(0);

    // Get service status
    let query_output = Command::new("sc.exe")
        .args(["query", name])
        .output()
        .map_err(|e| SnapshotError::Service(e.to_string()))?;

    let query = String::from_utf8_lossy(&query_output.stdout);
    let status = parse_sc_state(&query).unwrap_or(0);

    // Get description
    let desc_output = Command::new("sc.exe")
        .args(["qdescription", name])
        .output()
        .ok();
    let description = desc_output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.lines()
                .find(|l| l.contains("DESCRIPTION"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        });

    Ok(ServiceState {
        name: name.to_string(),
        start_type,
        status,
        description,
    })
}

fn restore_service_state(name: &str, state: &ServiceState) -> Result<(), SnapshotError> {
    use std::process::Command;

    // Set start type
    let start_type_str = match state.start_type {
        2 => "auto",
        3 => "demand",
        4 => "disabled",
        _ => "demand",
    };
    let _ = Command::new("sc.exe")
        .args(["config", name, "start=", start_type_str])
        .status();

    // Restore status (start/stop)
    if state.status == 4 { // RUNNING
        let _ = Command::new("sc.exe").args(["start", name]).status();
    } else if state.status == 1 { // STOPPED
        let _ = Command::new("sc.exe").args(["stop", name]).status();
    }

    Ok(())
}

fn parse_registry_path(path: &str) -> Result<(winreg::HKEY, String), SnapshotError> {
    let upper = path.to_uppercase();
    if upper.starts_with("HKLM\\") {
        Ok((winreg::enums::HKEY_LOCAL_MACHINE, path[5..].to_string()))
    } else if upper.starts_with("HKCU\\") {
        Ok((winreg::enums::HKEY_CURRENT_USER, path[5..].to_string()))
    } else if upper.starts_with("HKCR\\") {
        Ok((winreg::enums::HKEY_CLASSES_ROOT, path[5..].to_string()))
    } else if upper.starts_with("HKU\\") {
        Ok((winreg::enums::HKEY_USERS, path[4..].to_string()))
    } else {
        Err(SnapshotError::MissingData(format!("Invalid registry path: {}", path)))
    }
}

fn parse_sc_start_type(config: &str) -> Option<u32> {
    for line in config.lines() {
        if line.contains("START_TYPE") {
            return line.split(':').nth(1).and_then(|s| s.trim().parse().ok());
        }
    }
    None
}

fn parse_sc_state(query: &str) -> Option<u32> {
    for line in query.lines() {
        if line.contains("STATE") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                return parts[3].parse().ok();
            }
        }
    }
    None
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parse_registry_paths() {
        assert_eq!(parse_registry_path("HKLM\\SOFTWARE\\Test").unwrap().0, winreg::enums::HKEY_LOCAL_MACHINE);
        assert_eq!(parse_registry_path("HKCU\\SOFTWARE\\Test").unwrap().0, winreg::enums::HKEY_CURRENT_USER);
    }
}