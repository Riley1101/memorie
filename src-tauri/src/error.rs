use kalosm_llama::LlamaSourceError;
use rbert::{BertError, BertLoadingError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileError {
    #[error("File I/O error")]
    Io(#[from] std::io::Error),

    #[error("Home dir Directory not found")]
    HomeDirNotFound,
}

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("File I/O error")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Error getting config yaml: {0}")]
    FilePathError(#[from] FileError),
}

#[derive(Error, Debug)]
pub enum LlamaError {
    #[error("Llama source error: error loading model")]
    LlamaSource(#[from] LlamaSourceError),

    #[error("Llama chat error: {0}")]
    LlamaChat(String),

    #[error("Error getting ModelPath path: {0}")]
    FilePathError(#[from] FileError),
}

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("SurrealDB error: {0}")]
    SurrealDB(#[from] surrealdb::Error),

    #[error("Document table creation error")]
    DocumentCreation(#[from] kalosm::language::DocumentTableCreationError),

    #[error("Error getting DB path: {0}")]
    FilePathError(#[from] FileError),

    #[error("Embedding model: Bert error: {0}")]
    BertModelError(#[from] BertError),

    #[error("Embedding model: Bert Loading Error : {0}")]
    BertModelLoadingError(#[from] BertLoadingError),

    #[error("Embedding model not loaded")]
    ModelNotLoaded,
}

#[derive(Error, Debug)]
pub enum ChatError {
    #[error("Chat error: {0}")]
    Chat(#[from] LlamaError),
}
