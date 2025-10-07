mod commands;
mod config;
mod error;
mod fs;
mod undotree;

use config::AppConfig;
use std::sync::Mutex;
use undotree::UndoTree;

pub struct AppState {
    config: Mutex<AppConfig>,
    undotree: Mutex<UndoTree>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_config = AppConfig::load("config.yaml").expect("Failed to load config");

    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();

    let mut builder = tauri::Builder::default();

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(devtools);
    }

    let undo_tree = UndoTree::load(&app_config.undotree_dir).unwrap_or_else(|e| {
        eprintln!("Failed to load history, starting fresh: {}", e);
        UndoTree::new()
    });

    let app_state = AppState {
        config: Mutex::new(app_config),
        undotree: Mutex::new(undo_tree),
    };

    builder
        .manage(app_state)
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_files,
            commands::create_file,
            commands::update_file,
            commands::delete_file,
            commands::read_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
