use crate::error::FileError;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_EXTENSION: &str = "lexical";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct File {
    path: PathBuf,
    name: String,
}

/// Discovers files with the default extension in a given directory.
pub fn discover_files(directory: &Path) -> Result<Vec<File>, FileError> {
    let mut files = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();

        if path.is_file() && name.ends_with(DEFAULT_EXTENSION) {
            files.push(File { path, name });
        }
    }

    // Sort files for consistent test results
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Creates a new file with the given content.
pub fn create_file(path: &Path, content: &str) -> Result<File, FileError> {
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_string();

    fs::write(path, content)?;

    Ok(File {
        path: path.to_path_buf(),
        name,
    })
}

/// Updates the content of an existing file.
pub fn update_file(file: &File, content: &str) -> Result<File, FileError> {
    fs::write(&file.path, content)?;

    Ok(File {
        path: file.path.clone(),
        name: file.name.clone(),
    })
}

/// Deletes a file.
pub fn delete_file(file: &File) -> Result<(), FileError> {
    fs::remove_file(&file.path)?;

    Ok(())
}

#[cfg(test)]
mod fs_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.lexical");
        let content = "Hello, World!";

        let file = create_file(&file_path, content).unwrap();

        assert_eq!(file.name, "test.lexical");
        assert_eq!(file.path, file_path);
        assert!(file_path.exists());

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_discover_files() {
        let dir = tempdir().unwrap();

        // Create some files to be discovered
        create_file(&dir.path().join("a.lexical"), "content a").unwrap();
        create_file(&dir.path().join("b.lexical"), "content b").unwrap();

        // Create some files that should NOT be discovered
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

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, updated_content);
    }

    #[test]
    fn test_delete_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("delete_me.lexical");

        let file = create_file(&file_path, "I am temporary.").unwrap();
        assert!(file_path.exists());

        delete_file(&file).unwrap();
        assert!(!file_path.exists());
    }
}
