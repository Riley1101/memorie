use super::error::FileError;
use std::path::PathBuf;
use serde::{ Serialize, Deserialize};

/// Get the directory where the app stores its data.
pub fn get_app_dir() -> Result<PathBuf, FileError> {
    let home_dir = dirs::home_dir().ok_or(FileError::HomeDirNotFound)?;
    let app_dir = home_dir.join(".memorie/");
    Ok(app_dir)
}

/// Modes for chat interactions.
#[derive( Deserialize, Serialize, Clone, PartialEq)]
pub enum ChatMode {
    Normal,
    Autocomplete,
}

/// Convert ChatMode to its string representation.
impl ChatMode {
    pub fn as_str(&self) -> &str {
        match self {
            ChatMode::Normal => "Normal",
            ChatMode::Autocomplete => "Autocomplete",
        }
    }
}
