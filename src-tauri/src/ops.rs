use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Recursively finds all Markdown files (`.md`) in a given directory.
pub fn discover_markdown_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut found_files = Vec::new();

    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(extension) = entry.path().extension() {
                if extension == "md" {
                    found_files.push(entry.path().to_path_buf());
                }
            }
        }
    }
    Ok(found_files)
}

/// Reads the entire content of a file into a string.
pub fn read_file(path: &Path) -> anyhow::Result<String> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

/// Creates a new file with the given content.
///
/// Fails if the file already exists or if parent directories cannot be created.
pub fn create_file(path: &Path, content: &str) -> anyhow::Result<()> {
    if path.exists() {
        return Err(anyhow::anyhow!("File already exists: {:?}", path));
    }
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Updates an existing file, overwriting its content.
///
/// Fails if the file does not exist.
pub fn update_file(path: &Path, content: &str) -> anyhow::Result<()> {
    if !path.exists() {
        return Err(anyhow::anyhow!("File not found: {:?}", path));
    }
    fs::write(path, content)?;
    Ok(())
}

/// Saves a file, creating it if it doesn't exist or overwriting it if it does.
///
/// This is a convenient "upsert" operation.
pub fn save_file(path: &Path, content: &str) -> anyhow::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Deletes a file.
///
/// Fails if the file doesn't exist.
pub fn delete_file(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Err(anyhow::anyhow!("File not found for deletion: {:?}", path));
    }
    fs::remove_file(path)?;
    Ok(())
}
