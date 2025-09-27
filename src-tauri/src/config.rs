use anyhow;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub content_directory: PathBuf,
}

impl AppConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let config_str = fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&config_str)?;

        if !config.content_directory.exists() {
            fs::create_dir_all(&config.content_directory)?;
        }

        Ok(config)
    }
}

#[cfg(test)]
mod config_tests {
    use super::AppConfig;

    #[test]
    fn test_load_config() {
        let config = AppConfig::load("config.yaml").unwrap();
        assert!(config.content_directory.exists());
    }
}
