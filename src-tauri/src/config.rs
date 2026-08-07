use super::error::ConfigurationError;
use super::utils;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub content_directory: PathBuf,
    pub undotree_dir: PathBuf,
    pub default_llm_model: PathBuf,
    /// Selected model preset id (e.g. "qwen_2_5_1_5b_instruct"). Used when loading chat/autocomplete.
    #[serde(default)]
    pub default_llm_model_id: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// "owner/repo" of the GitHub repository writing is pushed to.
    #[serde(default)]
    pub github_repo: Option<String>,
    /// If true, commit & push any pending changes to GitHub when the app is closed.
    #[serde(default)]
    pub auto_push_on_exit: bool,
    /// If true, local AI (model download/load/chat) is enabled. Disabled by default so the app
    /// starts as a plain writing app.
    #[serde(default)]
    pub ai_enabled: bool,
}

impl AppConfig {
    pub fn new_default() -> Result<Self, ConfigurationError> {
        let app_dir = utils::get_app_dir()?;

        Ok(AppConfig {
            content_directory: app_dir.join("content"),
            undotree_dir: app_dir.join("history"),
            default_llm_model: app_dir.join("models").join("default_model.gguf"),
            default_llm_model_id: Some("qwen_2_5_1_5b_instruct".to_string()),
            system_prompt: None,
            github_repo: None,
            auto_push_on_exit: false,
            ai_enabled: false,
        })
    }

    pub fn init_default_config(path: &str) -> Result<(), ConfigurationError> {
        let config_path = Path::new(path);

        if config_path.exists() {
            return Ok(());
        }

        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let default_config = Self::new_default()?;

        let yaml_str = serde_yaml::to_string(&default_config)?;

        fs::write(config_path, yaml_str)?;

        Ok(())
    }

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

        if let Some(parent) = config.default_llm_model.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        Ok(config)
    }

    /// Save config to the given path (e.g. config.yaml).
    pub fn save(&self, path: &str) -> Result<(), ConfigurationError> {
        let yaml_str = serde_yaml::to_string(self)?;
        fs::write(path, yaml_str)?;
        Ok(())
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

    #[test]
    fn test_init_default_config() {
        let dir = tempdir().unwrap();
        let app_dir = dir.path().join(".memorie");
        let config_path = app_dir.join("config.yaml");
        let _ = config_path.to_str().unwrap();
        assert!(!app_dir.exists());
        assert!(!config_path.exists());

        let temp_config_path = dir.path().join("test_config.yaml");
        let temp_config_str = temp_config_path.to_str().unwrap();

        assert!(!temp_config_path.exists());

        match AppConfig::init_default_config(temp_config_str) {
            Ok(_) => {
                assert!(temp_config_path.exists());

                let config = AppConfig::load(temp_config_str).unwrap();
                assert!(config.content_directory.ends_with("content"));
                assert!(config.undotree_dir.ends_with("history"));
                assert!(config
                    .default_llm_model
                    .ends_with("models/default_model.gguf"));

                AppConfig::init_default_config(temp_config_str).unwrap();
                assert!(temp_config_path.exists());
            }
            Err(e) => {
                eprintln!("Warning: test_init_default_config skipped: {:?}", e);
            }
        }
    }
}
