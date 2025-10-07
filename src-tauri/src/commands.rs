use tauri::State;

use crate::fs::{self, File};
use crate::AppState;

#[tauri::command]
pub fn list_files(state: State<AppState>) -> Result<Vec<File>, String> {
    let config = state.config.lock().unwrap();
    let content_dir = &config.content_directory;
    let result = fs::discover_files(content_dir).map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn create_file(name: String, content: &str, state: State<AppState>) -> Result<File, String> {
    let config = state.config.lock().unwrap();
    let path = config.content_directory.join(&name);
    let result = fs::create_file(&path, content).map_err(|e| e.to_string())?;

    let mut undo_tree = state.undotree.lock().unwrap();

    undo_tree.add_change(&name, content);
    undo_tree
        .save(&config.undotree_dir)
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn update_file(name: String, content: &str, state: State<AppState>) -> Result<File, String> {
    let config = state.config.lock().unwrap();
    let path = config.content_directory.join(&name);
    println!("Updating file at path: {:?}", path);

    let file_to_update = File { path, name };

    let result = fs::update_file(&file_to_update, content).map_err(|e| e.to_string());

    let mut undo_tree = state.undotree.lock().unwrap();
    undo_tree.add_change(&file_to_update.name, content);
    undo_tree
        .save(&config.undotree_dir)
        .map_err(|e| e.to_string())
        .unwrap_or_else(|e| eprintln!("Failed to save undo tree: {}", e));
    result
}

#[tauri::command]
pub fn delete_file(name: String, state: State<AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap();
    let path = config.content_directory.join(&name);

    let file_to_delete = File { path, name };

    fs::delete_file(&file_to_delete).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_file(name: String, state: State<AppState>) -> Result<String, String> {
    let config = state.config.lock().unwrap();
    let path = config.content_directory.join(&name);
    let mut undo_tree = state.undotree.lock().unwrap();

    let file = File { path, name };

    file.read_content().map_err(|e| e.to_string())
}
