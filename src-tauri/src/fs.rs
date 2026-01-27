use crate::error::FileError;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const DEFAULT_EXTENSION: &str = "md";

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

pub fn get_recent(directory: &Path) -> Result<Vec<File>, FileError> {
    let mut files_with_mod_time: Vec<(PathBuf, SystemTime)> = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some(DEFAULT_EXTENSION) {
            let metadata = entry.metadata()?;
            let modified_time = metadata.modified()?;
            files_with_mod_time.push((path, modified_time));
        }
    }

    // Sort files by modification time in descending order
    files_with_mod_time.sort_by(|a, b| b.1.cmp(&a.1));

    let files = files_with_mod_time
        .into_iter()
        .map(|(path, _)| File::new(path))
        .collect();

    Ok(files)
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

pub fn rename_file(old_path: &Path, new_name: &str) -> Result<File, FileError> {
    let new_path = old_path.parent().unwrap_or(Path::new("")).join(new_name);
    fs::rename(old_path, &new_path)?;
    Ok(File::new(new_path))
}

#[cfg(test)]
mod fs_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_new_and_read_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_read.md");
        let content = "You should be able to read this.";
        fs::write(&file_path, content).unwrap();

        let file = File::new(file_path);
        assert_eq!(file.name, "test_read.md");
        assert_eq!(file.read_content().unwrap(), content);
    }

    #[test]
    fn test_file_extension() {
        let file_path = PathBuf::from("/tmp/test.md");
        let file = File::new(file_path);
        assert_eq!(file.extension(), Some("md"));

        let no_ext_path = PathBuf::from("/tmp/test");
        let no_ext_file = File::new(no_ext_path);
        assert_eq!(no_ext_file.extension(), None);
    }

    #[test]
    fn test_create_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        let content = "Hello, World!";

        let file = create_file(&file_path, content).unwrap();

        assert_eq!(file.name, "test.md");
        assert_eq!(file.path, file_path);
        assert!(file_path.exists());

        let read_content = file.read_content().unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_discover_files() {
        let dir = tempdir().unwrap();

        create_file(&dir.path().join("a.md"), "content a").unwrap();
        create_file(&dir.path().join("b.md"), "content b").unwrap();

        create_file(&dir.path().join("c.txt"), "content c").unwrap();
        fs::create_dir(dir.path().join("subfolder")).unwrap();

        let discovered = discover_files(dir.path()).unwrap();

        assert_eq!(discovered.len(), 2);
        assert_eq!(discovered[0].name, "a.md");
        assert_eq!(discovered[1].name, "b.md");
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
        let file_path = dir.path().join("update_me.md");
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
        let file_path = dir.path().join("delete_me.md");

        let file = create_file(&file_path, "I am temporary.").unwrap();
        assert!(file.path.exists());

        delete_file(&file).unwrap();
        assert!(!file.path.exists());
    }
    #[test]
    fn test_get_recent_files() {
        let dir = tempdir().unwrap();
        let file_path1 = dir.path().join("a.md");
        let file_path2 = dir.path().join("b.md");
        let file_path3 = dir.path().join("c.md");

        // Create files with a small delay to ensure different modification times
        create_file(&file_path1, "content a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_file(&file_path2, "content b").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_file(&file_path3, "content c").unwrap();

        let recent_files = get_recent(dir.path()).unwrap();

        assert_eq!(recent_files.len(), 3);
        assert_eq!(recent_files[0].name, "c.md");
        assert_eq!(recent_files[1].name, "b.md");
        assert_eq!(recent_files[2].name, "a.md");
    }
}
