use super::error::ConfigurationError;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub content_directory: PathBuf,
    pub undotree_dir: PathBuf,

    pub default_llm_model: PathBuf,
}

impl AppConfig {
    pub fn load(path: &str) -> Result<Self, ConfigurationError> {
        let config_str = fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&config_str)?;

        if !config.content_directory.exists() {
            fs::create_dir_all(&config.content_directory)?;
        }

        if let Some(parent) = config.undotree_dir.parent() {
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
        let undotree_path = dir.path().join("history");
        let default_llm_model = dir.path().join("default_llm_model");

        let yaml_content = format!(
            "content_directory: {:?}\nundotree_dir: {:?}\ndefault_llm_model: {:?}",
            content_dir, undotree_path, default_llm_model
        );

        fs::write(&config_path, yaml_content).unwrap();
        let config = AppConfig::load(config_path.to_str().unwrap()).unwrap();

        assert!(config.content_directory.exists());
        assert!(config.undotree_dir.parent().unwrap().exists());
    }
}
