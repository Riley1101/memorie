use anyhow;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub content_directory: PathBuf,
}

impl AppConfig {
    /// Loads configuration from the "config.yaml" file.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let config_str = fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&config_str)?;

        // Ensure the content directory exists, creating it if it doesn't.
        if !config.content_directory.exists() {
            fs::create_dir_all(&config.content_directory)?;
        }

        Ok(config)
    }
}
