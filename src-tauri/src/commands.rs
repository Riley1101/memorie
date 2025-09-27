use tauri::State;

use crate::fs::{self, File};
use crate::AppState;

#[tauri::command]
pub fn discover_files(state: State<AppState>) -> Result<Vec<File>, String> {
    let config = state.0.lock().unwrap();
    let content_dir = &config.content_directory;
    fs::discover_files(content_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_file(name: String, content: &str, state: State<AppState>) -> Result<File, String> {
    let config = state.0.lock().unwrap();
    let path = config.content_directory.join(&name);
    fs::create_file(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_file(name: String, content: &str, state: State<AppState>) -> Result<File, String> {
    let config = state.0.lock().unwrap();
    let path = config.content_directory.join(&name);

    let file_to_update = File { path, name };

    fs::update_file(&file_to_update, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_file(name: String, state: State<AppState>) -> Result<(), String> {
    let config = state.0.lock().unwrap();
    let path = config.content_directory.join(&name);

    let file_to_delete = File { path, name };

    fs::delete_file(&file_to_delete).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_file(name: String, state: State<AppState>) -> Result<String, String> {
    let config = state.0.lock().unwrap();
    let path = config.content_directory.join(&name);

    let file = File { path, name };

    file.read_content().map_err(|e| e.to_string())
}
