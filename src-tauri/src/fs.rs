use crate::error::FileError;
use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const DEFAULT_EXTENSION: &str = "md";
const MAX_READ_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
const READ_CONCURRENCY: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct File {
    pub path: PathBuf,
    pub name: String,
    pub last_modified: u64,
    pub folder: Option<String>,
}

impl File {
    pub fn new(path: PathBuf, modified: Option<SystemTime>) -> Self {
        Self::with_relative_dir(path, modified, Vec::new())
    }

    /// `folder`, if given, is treated as the single top-level directory the file
    /// lives in. Kept for callers that only ever deal with one level of nesting.
    pub fn with_folder(path: PathBuf, modified: Option<SystemTime>, folder: Option<String>) -> Self {
        Self::with_relative_dir(path, modified, folder.into_iter().collect())
    }

    /// `relative_dir` is the chain of directory names from the content root down
    /// to (but not including) the file itself, e.g. `["Novel", "Chapter 1"]` for
    /// `Novel/Chapter 1/Scene 1.md`. `folder` is set to the first segment (the
    /// binder), matching the flat binder model callers filter by.
    pub fn with_relative_dir(
        path: PathBuf,
        modified: Option<SystemTime>,
        relative_dir: Vec<String>,
    ) -> Self {
        let base_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();

        let name = if relative_dir.is_empty() {
            base_name
        } else {
            format!("{}/{}", relative_dir.join("/"), base_name)
        };

        let folder = relative_dir.into_iter().next();

        let last_modified = modified
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        File { path, name, last_modified, folder }
    }

    pub fn read_content(&self) -> Result<String, FileError> {
        let size = fs::metadata(&self.path)?.len();
        if size > MAX_READ_BYTES {
            return Err(FileError::FileTooLarge(size, MAX_READ_BYTES));
        }
        let content = fs::read_to_string(&self.path)?;
        Ok(content)
    }

    #[allow(dead_code)]
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(OsStr::to_str)
    }
}

/// Recursively scans `directory` for `.md` files at any depth. Top-level
/// subdirectories are binders; folders nested further (chapters, scenes, ...)
/// are just part of the tree inside a binder. Dotfiles/dirs are skipped.
fn scan_dir_recursive(
    current: &Path,
    relative_dir: &mut Vec<String>,
    found: &mut Vec<(PathBuf, SystemTime, Vec<String>)>,
) -> Result<(), FileError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some(DEFAULT_EXTENSION) {
            let modified = metadata.modified()?;
            found.push((path, modified, relative_dir.clone()));
        } else if metadata.is_dir() {
            let dir_name = match path.file_name().and_then(OsStr::to_str) {
                Some(n) if !n.starts_with('.') => n.to_string(),
                _ => continue,
            };

            relative_dir.push(dir_name);
            scan_dir_recursive(&path, relative_dir, found)?;
            relative_dir.pop();
        }
    }

    Ok(())
}

fn scan_files(directory: &Path) -> Result<Vec<(PathBuf, SystemTime, Vec<String>)>, FileError> {
    let mut found = Vec::new();
    let mut relative_dir = Vec::new();
    scan_dir_recursive(directory, &mut relative_dir, &mut found)?;
    Ok(found)
}

pub fn get_recent(directory: &Path) -> Result<Vec<File>, FileError> {
    let mut files_with_mod_time = scan_files(directory)?;

    // Sort files by modification time in descending order
    files_with_mod_time.sort_by(|a, b| b.1.cmp(&a.1));

    let files = files_with_mod_time
        .into_iter()
        .map(|(path, mod_time, relative_dir)| File::with_relative_dir(path, Some(mod_time), relative_dir))
        .collect();

    Ok(files)
}

/// Reads the content of many files concurrently, bounded to `READ_CONCURRENCY`
/// in-flight reads at a time. Each file's read runs on the blocking threadpool
/// since `File::read_content` is a synchronous std::fs call.
pub async fn read_contents_batch(files: Vec<File>) -> Vec<(File, Result<String, FileError>)> {
    stream::iter(files)
        .map(|file| async move {
            let result = tokio::task::spawn_blocking({
                let file = file.clone();
                move || file.read_content()
            })
            .await
            .unwrap_or_else(|e| Err(FileError::ContentConversionError(e.to_string())));
            (file, result)
        })
        .buffer_unordered(READ_CONCURRENCY)
        .collect()
        .await
}

pub fn discover_files(directory: &Path) -> Result<Vec<File>, FileError> {
    let found = scan_files(directory)?;

    let mut files: Vec<File> = found
        .into_iter()
        .map(|(path, modified, relative_dir)| File::with_relative_dir(path, Some(modified), relative_dir))
        .collect();

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// `relative_dir` is the full chain of directory names (binder, then any nested
/// chapter/scene folders) the file lives in, e.g. `["Novel", "Chapter 1"]`.
pub fn create_file(path: &Path, content: &str, relative_dir: Vec<String>) -> Result<File, FileError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    let modified = fs::metadata(path)?.modified().ok();
    Ok(File::with_relative_dir(path.to_path_buf(), modified, relative_dir))
}

/// Creates an empty folder (binder subdirectory, chapter, scene grouping, ...)
/// at `relative_path` under `directory`, creating any missing parents.
pub fn create_folder(directory: &Path, relative_path: &str) -> Result<(), FileError> {
    fs::create_dir_all(directory.join(relative_path))?;
    Ok(())
}

/// Recursively lists every directory under `directory` (including top-level
/// binders), as `/`-joined paths relative to `directory`, e.g. `"Novel/Chapter 2"`.
/// Used so folders with no writings in them yet still show up in a binder's tree.
pub fn list_folders(directory: &Path) -> Result<Vec<String>, FileError> {
    let mut folders = Vec::new();
    let mut relative_dir = Vec::new();
    collect_folders(directory, &mut relative_dir, &mut folders)?;
    folders.sort();
    Ok(folders)
}

fn collect_folders(
    current: &Path,
    relative_dir: &mut Vec<String>,
    out: &mut Vec<String>,
) -> Result<(), FileError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();

        if entry.metadata()?.is_dir() {
            let dir_name = match path.file_name().and_then(OsStr::to_str) {
                Some(n) if !n.starts_with('.') => n.to_string(),
                _ => continue,
            };

            relative_dir.push(dir_name);
            out.push(relative_dir.join("/"));
            collect_folders(&path, relative_dir, out)?;
            relative_dir.pop();
        }
    }

    Ok(())
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

pub fn rename_binder(directory: &Path, old_name: &str, new_name: &str) -> Result<(), FileError> {
    fs::rename(directory.join(old_name), directory.join(new_name))?;
    Ok(())
}

/// Deletes binder `name` inside `directory`. Refuses if the binder contains
/// any writings (files), so a binder can only be deleted while empty.
pub fn delete_binder(directory: &Path, name: &str) -> Result<(), FileError> {
    let path = directory.join(name);

    if fs::read_dir(&path)?.next().is_some() {
        return Err(FileError::BinderNotEmpty(name.to_string()));
    }

    fs::remove_dir(&path)?;
    Ok(())
}

/// Deletes folder `relative_path` (a chapter/scene grouping nested inside a
/// binder) inside `directory`. Refuses if it contains anything, same rule as
/// `delete_binder`.
pub fn delete_folder(directory: &Path, relative_path: &str) -> Result<(), FileError> {
    let path = directory.join(relative_path);

    if fs::read_dir(&path)?.next().is_some() {
        return Err(FileError::BinderNotEmpty(relative_path.to_string()));
    }

    fs::remove_dir(&path)?;
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

/// Moves a writing to any content-relative path, creating parent folders as
/// needed. Refuses to overwrite an existing file.
pub fn move_file(directory: &Path, old_name: &str, new_name: &str) -> Result<File, FileError> {
    let old_path = directory.join(old_name);
    let new_path = directory.join(new_name);
    if new_path.exists() {
        return Err(FileError::AlreadyExists(new_name.to_string()));
    }
    if let Some(parent) = new_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&old_path, &new_path)?;
    let modified = fs::metadata(&new_path)?.modified().ok();
    Ok(File::new(new_path, modified))
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

        let file = create_file(&file_path, content, Vec::new()).unwrap();

        assert_eq!(file.name, "test.md");
        assert_eq!(file.path, file_path);
        assert!(file_path.exists());

        let read_content = file.read_content().unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_discover_files() {
        let dir = tempdir().unwrap();

        create_file(&dir.path().join("a.md"), "content a", Vec::new()).unwrap();
        create_file(&dir.path().join("b.md"), "content b", Vec::new()).unwrap();

        create_file(&dir.path().join("c.txt"), "content c", Vec::new()).unwrap();

        let discovered = discover_files(dir.path()).unwrap();

        assert_eq!(discovered.len(), 2);
        assert_eq!(discovered[0].name, "a.md");
        assert_eq!(discovered[1].name, "b.md");
    }

    #[test]
    fn test_discover_files_in_binder() {
        let dir = tempdir().unwrap();

        create_file(&dir.path().join("top.md"), "top level", Vec::new()).unwrap();
        fs::create_dir(dir.path().join("Binder")).unwrap();
        create_file(
            &dir.path().join("Binder").join("nested.md"),
            "nested",
            vec!["Binder".to_string()],
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
    fn test_discover_files_deeply_nested() {
        let dir = tempdir().unwrap();

        create_file(
            &dir.path().join("Novel").join("Chapter 1").join("Scene 1.md"),
            "scene one",
            vec!["Novel".to_string(), "Chapter 1".to_string()],
        )
        .unwrap();

        let discovered = discover_files(dir.path()).unwrap();

        assert_eq!(discovered.len(), 1);
        let scene = &discovered[0];
        assert_eq!(scene.name, "Novel/Chapter 1/Scene 1.md");
        assert_eq!(scene.folder.as_deref(), Some("Novel"));
    }

    #[test]
    fn test_create_folder() {
        let dir = tempdir().unwrap();

        create_folder(dir.path(), "Novel/Chapter 1").unwrap();
        assert!(dir.path().join("Novel").join("Chapter 1").is_dir());
    }

    #[test]
    fn test_list_folders_includes_empty_nested_folders() {
        let dir = tempdir().unwrap();

        create_folder(dir.path(), "Novel/Chapter 1").unwrap();
        create_folder(dir.path(), "Novel/Chapter 2").unwrap();

        let folders = list_folders(dir.path()).unwrap();

        assert_eq!(
            folders,
            vec![
                "Novel".to_string(),
                "Novel/Chapter 1".to_string(),
                "Novel/Chapter 2".to_string(),
            ]
        );
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
    fn test_rename_binder() {
        let dir = tempdir().unwrap();
        create_binder(dir.path(), "Ideas").unwrap();

        rename_binder(dir.path(), "Ideas", "Notes").unwrap();
        assert_eq!(list_binders(dir.path()).unwrap(), vec!["Notes".to_string()]);
    }

    #[test]
    fn test_delete_empty_binder() {
        let dir = tempdir().unwrap();
        create_binder(dir.path(), "Ideas").unwrap();

        delete_binder(dir.path(), "Ideas").unwrap();
        assert!(list_binders(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn test_delete_binder_with_writings_fails() {
        let dir = tempdir().unwrap();
        create_binder(dir.path(), "Ideas").unwrap();
        create_file(
            &dir.path().join("Ideas").join("note.md"),
            "content",
            vec!["Ideas".to_string()],
        )
        .unwrap();

        let err = delete_binder(dir.path(), "Ideas").unwrap_err();
        assert!(matches!(err, FileError::BinderNotEmpty(_)));
        assert_eq!(list_binders(dir.path()).unwrap(), vec!["Ideas".to_string()]);
    }

    #[test]
    fn test_delete_empty_folder() {
        let dir = tempdir().unwrap();
        create_folder(dir.path(), "Novel/Chapter 1").unwrap();

        delete_folder(dir.path(), "Novel/Chapter 1").unwrap();
        assert_eq!(list_folders(dir.path()).unwrap(), vec!["Novel".to_string()]);
    }

    #[test]
    fn test_delete_folder_with_scenes_fails() {
        let dir = tempdir().unwrap();
        create_file(
            &dir.path().join("Novel").join("Chapter 1").join("Scene 1.md"),
            "content",
            vec!["Novel".to_string(), "Chapter 1".to_string()],
        )
        .unwrap();

        let err = delete_folder(dir.path(), "Novel/Chapter 1").unwrap_err();
        assert!(matches!(err, FileError::BinderNotEmpty(_)));
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

        let file = create_file(&file_path, initial_content, Vec::new()).unwrap();
        let updated_file = update_file(&file, updated_content).unwrap();

        assert_eq!(file, updated_file);

        let read_content = updated_file.read_content().unwrap();
        assert_eq!(read_content, updated_content);
    }

    #[test]
    fn test_read_content_too_large() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("big.md");
        fs::write(&file_path, vec![b'a'; (MAX_READ_BYTES + 1) as usize]).unwrap();

        let file = File::new(file_path, None);
        let err = file.read_content().unwrap_err();
        assert!(matches!(err, FileError::FileTooLarge(_, _)));
    }

    #[test]
    fn test_delete_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("delete_me.md");

        let file = create_file(&file_path, "I am temporary.", Vec::new()).unwrap();
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
        create_file(&file_path1, "content a", Vec::new()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_file(&file_path2, "content b", Vec::new()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_file(&file_path3, "content c", Vec::new()).unwrap();

        let recent_files = get_recent(dir.path()).unwrap();

        assert_eq!(recent_files.len(), 3);
        assert_eq!(recent_files[0].name, "c.md");
        assert_eq!(recent_files[1].name, "b.md");
        assert_eq!(recent_files[2].name, "a.md");
    }
}
