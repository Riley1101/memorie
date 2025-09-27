mod commands;
mod config;
mod error;
mod fs;
mod store;

use config::AppConfig;
use std::sync::Mutex;

pub struct AppState(pub Mutex<AppConfig>);

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

    let db_path = app_config.database_path.clone();

    builder
        .manage(AppState(Mutex::new(app_config)))
        .plugin(tauri_plugin_opener::init())
        .plugin(store::init(&db_path).build())
        .invoke_handler(tauri::generate_handler![
            commands::discover_files,
            commands::create_file,
            commands::update_file,
            commands::delete_file,
            commands::read_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
