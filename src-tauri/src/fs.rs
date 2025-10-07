use crate::error::FileError;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_EXTENSION: &str = "lexical";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct File {
    pub path: PathBuf,
    pub name: String,
}

impl File {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();
        File { path, name }
    }

    pub fn read_content(&self) -> Result<String, FileError> {
        let content = fs::read_to_string(&self.path)?;
        Ok(content)
    }

    #[allow(dead_code)]
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(OsStr::to_str)
    }
}

pub fn discover_files(directory: &Path) -> Result<Vec<File>, FileError> {
    let mut files = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some(DEFAULT_EXTENSION) {
            files.push(File::new(path));
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

pub fn create_file(path: &Path, content: &str) -> Result<File, FileError> {
    fs::write(path, content)?;
    Ok(File::new(path.to_path_buf()))
}

pub fn update_file(file: &File, content: &str) -> Result<File, FileError> {
    fs::write(&file.path, content)?;
    Ok(file.clone())
}

pub fn delete_file(file: &File) -> Result<(), FileError> {
    fs::remove_file(&file.path)?;
    Ok(())
}

#[cfg(test)]
mod fs_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_new_and_read_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_read.lexical");
        let content = "You should be able to read this.";
        fs::write(&file_path, content).unwrap();

        let file = File::new(file_path);
        assert_eq!(file.name, "test_read.lexical");
        assert_eq!(file.read_content().unwrap(), content);
    }

    #[test]
    fn test_file_extension() {
        let file_path = PathBuf::from("/tmp/test.lexical");
        let file = File::new(file_path);
        assert_eq!(file.extension(), Some("lexical"));

        let no_ext_path = PathBuf::from("/tmp/test");
        let no_ext_file = File::new(no_ext_path);
        assert_eq!(no_ext_file.extension(), None);
    }

    #[test]
    fn test_create_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.lexical");
        let content = "Hello, World!";

        let file = create_file(&file_path, content).unwrap();

        assert_eq!(file.name, "test.lexical");
        assert_eq!(file.path, file_path);
        assert!(file_path.exists());

        let read_content = file.read_content().unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_discover_files() {
        let dir = tempdir().unwrap();

        create_file(&dir.path().join("a.lexical"), "content a").unwrap();
        create_file(&dir.path().join("b.lexical"), "content b").unwrap();

        create_file(&dir.path().join("c.txt"), "content c").unwrap();
        fs::create_dir(dir.path().join("subfolder")).unwrap();

        let discovered = discover_files(dir.path()).unwrap();

        assert_eq!(discovered.len(), 2);
        assert_eq!(discovered[0].name, "a.lexical");
        assert_eq!(discovered[1].name, "b.lexical");
    }

    #[test]
    fn test_discover_files_in_empty_directory() {
        let dir = tempdir().unwrap();
        let discovered = discover_files(dir.path()).unwrap();
        assert!(discovered.is_empty());
    }

    #[test]
    fn test_update_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("update_me.lexical");
        let initial_content = "Initial content.";
        let updated_content = "This content has been updated.";

        let file = create_file(&file_path, initial_content).unwrap();
        let updated_file = update_file(&file, updated_content).unwrap();

        assert_eq!(file, updated_file);

        let read_content = updated_file.read_content().unwrap();
        assert_eq!(read_content, updated_content);
    }

    #[test]
    fn test_delete_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("delete_me.lexical");

        let file = create_file(&file_path, "I am temporary.").unwrap();
        assert!(file.path.exists());

        delete_file(&file).unwrap();
        assert!(!file.path.exists());
    }
}
