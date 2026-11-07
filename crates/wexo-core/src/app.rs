use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Runtime information about the running application, shown in Settings/About.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub is_admin: bool,
    pub log_dir: String,
}
