mod commands;
mod config;
mod error;

mod fs;
mod llm;
mod memory;
mod prompts;
mod responses;
mod undotree;
mod utils;
mod workers;

use config::AppConfig;
use tauri::Manager;
use tokio::sync::Mutex;
use undotree::UndoTree;
use workers::LlmEventService;

use crate::llm::Model;
use crate::memory::Memory;

pub struct AppState {
    config: Mutex<AppConfig>,
    undotree: Mutex<UndoTree>,
    model: Mutex<Model>,
    memory: Mutex<Memory>,
    workers: LlmEventService,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let home_dir = utils::get_app_dir()
        .expect("Failed to get home directory")
        .join("config.yaml");

    let config_path_str = home_dir.to_str().expect("Config path is not valid UTF-8");

    if let Err(e) = config::AppConfig::init_default_config(config_path_str) {
        eprintln!("Failed to initialize config: {}", e);
        panic!("Failed to initialize config: {}", e);
    }

    let app_config = config::AppConfig::load(config_path_str)
        .expect("Failed to load config after initialization");

    let memory_instance = Memory::new()
        .await
        .expect("Failed to initialize memory database");

    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();

    let undo_tree = UndoTree::load(&app_config.undotree_dir).unwrap_or_else(|_| UndoTree::new());

    let default_model_path = app_config.default_llm_model.clone();

    let model = Model::new(default_model_path);

    let mut builder = tauri::Builder::default().setup(|app| {
        let app_handle = app.handle().clone();
        let app_state = AppState {
            config: Mutex::new(app_config),
            undotree: Mutex::new(undo_tree),
            model: Mutex::new(model),
            memory: Mutex::new(memory_instance),
            workers: LlmEventService::new(app_handle),
        };
        app.manage(app_state);
        Ok(())
    });

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(devtools);
    }

    builder
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_recents,
            commands::create_file,
            commands::update_file,
            commands::delete_file,
            commands::load_models,
            commands::run_chat,
            commands::cancel_chat,
            commands::read_file,
            commands::get_file_history,
            commands::create_document_context,
            commands::update_text_chunk,
            commands::get_document_context,
            commands::search_documents,
            commands::get_chat_sessions,
            commands::goto_file_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
