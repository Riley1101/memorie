use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use tauri::State;
use walkdir::WalkDir;

use crate::AppState;

const EXTENSION: &str = "lexical";

/// A serializable representation of a file, including its path and name.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    path: PathBuf,
    name: String,
}

#[tauri::command]
pub fn discover_files(state: State<AppState>) -> Result<Vec<FileEntry>, String> {
    let config = state.0.lock().unwrap();
    let root = &config.content_directory;
    let mut found_files = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() {
            if entry.path().extension().and_then(OsStr::to_str) == Some(EXTENSION) {
                found_files.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_path_buf(),
                });
            }
        }
    }
    Ok(found_files)
}

#[tauri::command]
pub fn read_file(state: State<'_, AppState>, filename: String) -> Result<String, String> {
    let config = state.0.lock().unwrap();
    let root = &config.content_directory;
    println!("Reading file: {}", filename);

    let full_path: PathBuf = root.join(filename);
    fs::read_to_string(&full_path)
        .map_err(|e| format!("Failed to read file at '{}': {}", full_path.display(), e))
}

#[tauri::command]
pub fn save_file(path: PathBuf, content: String) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| e.to_string())
}
