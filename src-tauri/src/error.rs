use tauri_plugin_sql::Error as SqlError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileError {
    #[error("File I/O error")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("File I/O error")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error")]
    Yaml(#[from] serde_yaml::Error),
}

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Invalid database path provided")]
    InvalidPath,

    #[error("An SQL error occurred: {0}")]
    Sql(#[from] SqlError),
}
