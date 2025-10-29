use super::error::FileError;
use std::path::PathBuf;

/// Get the directory where the app stores its data.
pub fn get_app_dir() -> Result<PathBuf, FileError> {
    let home_dir = dirs::home_dir().ok_or(FileError::HomeDirNotFound)?;
    let app_dir = home_dir.join(".memorie/");
    Ok(app_dir)
}
