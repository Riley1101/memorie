// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    // A GUI app launched from Finder inherits a bare PATH; without this, git and
    // other tools the app shells out to aren't found. Not fatal if it fails.
    if let Err(e) = fix_path_env::fix() {
        eprintln!("Could not inherit the shell PATH: {e}");
    }
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    memoire_lib::run().await;
}
