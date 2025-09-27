use super::error::ConfigurationError;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub content_directory: PathBuf,
    pub database_path: PathBuf,
}

impl AppConfig {
    pub fn load(path: &str) -> Result<Self, ConfigurationError> {
        let config_str = fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&config_str)?;

        if !config.content_directory.exists() {
            fs::create_dir_all(&config.content_directory)?;
        }

        if let Some(parent) = config.database_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod config_tests {
    use super::AppConfig;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_load_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        let content_dir = dir.path().join("content");
        let db_path = dir.path().join("db/test.db");

        let yaml_content = format!(
            "content_directory: {:?}\ndatabase_path: {:?}",
            content_dir, db_path
        );

        fs::write(&config_path, yaml_content).unwrap();
        let config = AppConfig::load(config_path.to_str().unwrap()).unwrap();

        assert!(config.content_directory.exists());
        assert!(config.database_path.parent().unwrap().exists());
    }
}
