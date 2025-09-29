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

