mod commands;
mod config;
mod error;
mod fs;
mod llm;
mod undotree;
mod utils;
mod memory;

use config::AppConfig;
use dirs;
use tokio::sync::Mutex;
use undotree::UndoTree;

use crate::llm::Model;
use crate::memory::Memory;

pub struct AppState {
    config: Mutex<AppConfig>,
    undotree: Mutex<UndoTree>,
    model: Mutex<Model>,
    memory: Mutex<Memory>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let home_dir = utils::get_app_dir()
        .expect("Failed to get home directory")
        .join("config.yaml");

    let memory_instance = Memory::new()
        .await
        .expect("Failed to initialize memory database");

    let app_config = AppConfig::load(home_dir.to_str().expect("Failed to get home directory"))
        .expect("Failed to load config");

    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();

    let mut builder = tauri::Builder::default();

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(devtools);
    }

    let undo_tree = UndoTree::load(&app_config.undotree_dir).unwrap_or_else(|_| UndoTree::new());

    let default_model_path = app_config.default_llm_model.clone();

    let model = Model::new(default_model_path);

    let app_state = AppState {
        config: Mutex::new(app_config),
        undotree: Mutex::new(undo_tree),
        model: Mutex::new(model),
        memory:Mutex::new(memory_instance),
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
            commands::load_model,
            commands::run_chat,
            commands::read_file,
            commands::create_embeddings,
            commands::search_embeddings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
