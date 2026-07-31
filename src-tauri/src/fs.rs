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
    pub last_modified: u64,
    pub folder: Option<String>,
}

impl File {
    pub fn new(path: PathBuf, modified: Option<SystemTime>) -> Self {
        Self::with_folder(path, modified, None)
    }

    pub fn with_folder(path: PathBuf, modified: Option<SystemTime>, folder: Option<String>) -> Self {
        let base_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();

        let name = match &folder {
            Some(dir) => format!("{}/{}", dir, base_name),
            None => base_name,
        };

        let last_modified = modified
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        File { path, name, last_modified, folder }
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

/// Scans `directory` for `.md` files, one level deep: top-level files (folder=None)
/// plus files directly inside each top-level subdirectory (folder=Some(dir_name)).
/// Binders are not nested, so sub-subdirectories are not scanned.
fn scan_files(directory: &Path) -> Result<Vec<(PathBuf, SystemTime, Option<String>)>, FileError> {
    let mut found = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some(DEFAULT_EXTENSION) {
            let modified = metadata.modified()?;
            found.push((path, modified, None));
        } else if metadata.is_dir() {
            let dir_name = match path.file_name().and_then(OsStr::to_str) {
                Some(n) if !n.starts_with('.') => n.to_string(),
                _ => continue,
            };

            for sub_entry in fs::read_dir(&path)? {
                let sub_entry = sub_entry?;
                let sub_path = sub_entry.path();

                if sub_path.is_file()
                    && sub_path.extension().and_then(OsStr::to_str) == Some(DEFAULT_EXTENSION)
                {
                    let sub_metadata = sub_entry.metadata()?;
                    let modified = sub_metadata.modified()?;
                    found.push((sub_path, modified, Some(dir_name.clone())));
                }
            }
        }
    }

    Ok(found)
}

pub fn get_recent(directory: &Path) -> Result<Vec<File>, FileError> {
    let mut files_with_mod_time = scan_files(directory)?;

    // Sort files by modification time in descending order
    files_with_mod_time.sort_by(|a, b| b.1.cmp(&a.1));

    let files = files_with_mod_time
        .into_iter()
        .map(|(path, mod_time, folder)| File::with_folder(path, Some(mod_time), folder))
        .collect();

    Ok(files)
}

pub fn discover_files(directory: &Path) -> Result<Vec<File>, FileError> {
    let found = scan_files(directory)?;

    let mut files: Vec<File> = found
        .into_iter()
        .map(|(path, modified, folder)| File::with_folder(path, Some(modified), folder))
        .collect();

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// `folder`, if given, is the binder (top-level subdirectory) name the file lives in.
pub fn create_file(path: &Path, content: &str, folder: Option<String>) -> Result<File, FileError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    let modified = fs::metadata(path)?.modified().ok();
    Ok(File::with_folder(path.to_path_buf(), modified, folder))
}

pub fn list_binders(directory: &Path) -> Result<Vec<String>, FileError> {
    let mut binders = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(OsStr::to_str) {
                if !name.starts_with('.') {
                    binders.push(name.to_string());
                }
            }
        }
    }

    binders.sort();
    Ok(binders)
}

pub fn create_binder(directory: &Path, name: &str) -> Result<(), FileError> {
    fs::create_dir_all(directory.join(name))?;
    Ok(())
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
    let modified = fs::metadata(&new_path)?.modified().ok();
    Ok(File::new(new_path, modified))
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

        let file = File::new(file_path, None);
        assert_eq!(file.name, "test_read.md");
        assert_eq!(file.read_content().unwrap(), content);
    }

    #[test]
    fn test_file_extension() {
        let file_path = PathBuf::from("/tmp/test.md");
        let file = File::new(file_path, None);
        assert_eq!(file.extension(), Some("md"));

        let no_ext_path = PathBuf::from("/tmp/test");
        let no_ext_file = File::new(no_ext_path, None);
        assert_eq!(no_ext_file.extension(), None);
    }

    #[test]
    fn test_create_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        let content = "Hello, World!";

        let file = create_file(&file_path, content, None).unwrap();

        assert_eq!(file.name, "test.md");
        assert_eq!(file.path, file_path);
        assert!(file_path.exists());

        let read_content = file.read_content().unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_discover_files() {
        let dir = tempdir().unwrap();

        create_file(&dir.path().join("a.md"), "content a", None).unwrap();
        create_file(&dir.path().join("b.md"), "content b", None).unwrap();

        create_file(&dir.path().join("c.txt"), "content c", None).unwrap();

        let discovered = discover_files(dir.path()).unwrap();

        assert_eq!(discovered.len(), 2);
        assert_eq!(discovered[0].name, "a.md");
        assert_eq!(discovered[1].name, "b.md");
    }

    #[test]
    fn test_discover_files_in_binder() {
        let dir = tempdir().unwrap();

        create_file(&dir.path().join("top.md"), "top level", None).unwrap();
        fs::create_dir(dir.path().join("Binder")).unwrap();
        create_file(
            &dir.path().join("Binder").join("nested.md"),
            "nested",
            Some("Binder".to_string()),
        )
        .unwrap();

        let discovered = discover_files(dir.path()).unwrap();

        assert_eq!(discovered.len(), 2);
        let nested = discovered.iter().find(|f| f.name == "Binder/nested.md").unwrap();
        assert_eq!(nested.folder.as_deref(), Some("Binder"));
        let top = discovered.iter().find(|f| f.name == "top.md").unwrap();
        assert_eq!(top.folder, None);
    }

    #[test]
    fn test_list_and_create_binder() {
        let dir = tempdir().unwrap();
        assert!(list_binders(dir.path()).unwrap().is_empty());

        create_binder(dir.path(), "Ideas").unwrap();
        let binders = list_binders(dir.path()).unwrap();
        assert_eq!(binders, vec!["Ideas".to_string()]);
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

        let file = create_file(&file_path, initial_content, None).unwrap();
        let updated_file = update_file(&file, updated_content).unwrap();

        assert_eq!(file, updated_file);

        let read_content = updated_file.read_content().unwrap();
        assert_eq!(read_content, updated_content);
    }

    #[test]
    fn test_delete_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("delete_me.md");

        let file = create_file(&file_path, "I am temporary.", None).unwrap();
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
        create_file(&file_path1, "content a", None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_file(&file_path2, "content b", None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_file(&file_path3, "content c", None).unwrap();

        let recent_files = get_recent(dir.path()).unwrap();

        assert_eq!(recent_files.len(), 3);
        assert_eq!(recent_files[0].name, "c.md");
        assert_eq!(recent_files[1].name, "b.md");
        assert_eq!(recent_files[2].name, "a.md");
    }
}
