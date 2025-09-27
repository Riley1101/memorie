use crate::fs::{self, File};
use std::path::PathBuf;

#[tauri::command]
pub fn discover_files(directory: PathBuf) -> Result<Vec<File>, String> {
    fs::discover_files(&directory).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_file(path: PathBuf, content: &str) -> Result<File, String> {
    fs::create_file(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_file(file: File, content: &str) -> Result<File, String> {
    fs::update_file(&file, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_file(file: File) -> Result<(), String> {
    fs::delete_file(&file).map_err(|e| e.to_string())
}
