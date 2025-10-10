use std::{error::Error, path::PathBuf};

/// Get the directory where the app stores its data.
pub fn get_app_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home_dir = dirs::home_dir().ok_or("Failed to get the home directory.")?;
    let app_dir = home_dir.join(".memorie/");
    Ok(app_dir)
}
