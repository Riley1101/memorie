// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fix_path_env;

#[tokio::main]
async fn main() {
    fix_path_env::fix();
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    memoire_lib::run().await;
}
