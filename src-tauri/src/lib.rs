mod command;
mod config;

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

    builder
        .manage(AppState(Mutex::new(app_config)))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            command::discover_files,
            command::read_file,
            command::save_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
