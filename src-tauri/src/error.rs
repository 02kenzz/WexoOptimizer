use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Serializable error returned by Tauri commands.
///
/// The frontend receives `{ code, message }`, never a Rust debug dump.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Config(#[from] crate::config::ConfigError),
}

impl CommandError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Config(_) => "config_error",
        }
    }
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CommandError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}
