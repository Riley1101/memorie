use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileError {
    #[error("File I/O error")]
    Io(#[from] std::io::Error),
}
