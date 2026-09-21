mod commands;
mod config;
mod dropbox;
mod error;
mod export;

mod fs;
mod git;
mod import;
mod llm;
mod memory;
mod pdf;
mod prompts;
mod providers;
mod responses;
mod rtf;
mod search;
mod undotree;
mod utils;
mod workers;

use config::{AppConfig, SyncProvider};
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
    memory: Memory,
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
            memory: memory_instance,
            workers: LlmEventService::new(app_handle),
        };
        app.manage(app_state);

        if let Some(window) = app.get_webview_window("main") {
            let app_handle = app.handle().clone();
            let closing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let window_for_close = window.clone();

            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if closing.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        // Already ran the auto-push check; let this close through.
                        return;
                    }
                    api.prevent_close();

                    let app_handle = app_handle.clone();
                    let window = window_for_close.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<AppState>();
                        // Only the preferred storage pushes on exit, so closing
                        // the app never fans the same writing out to two remotes.
                        let (should_push, content_dir, dropbox_folder) = {
                            let config = state.config.lock().await;
                            let provider = config.sync_provider.clone();
                            (
                                provider == SyncProvider::Github
                                    && config.auto_push_on_exit
                                    && config.github_repo.is_some(),
                                config.content_directory.clone(),
                                (provider == SyncProvider::Dropbox
                                    && config.dropbox_auto_push_on_exit)
                                    .then(|| config.dropbox_folder.clone())
                                    .flatten(),
                            )
                        };

                        if should_push {
                            if let Ok(Some(token)) = git::load_token() {
                                if let Ok(user) = git::fetch_github_user(&token).await {
                                    let author_name = user.name.unwrap_or_else(|| user.login.clone());
                                    let author_email =
                                        format!("{}@users.noreply.github.com", user.login);
                                    let message = format!(
                                        "Auto-sync on exit — {}",
                                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                                    );
                                    let _ = git::commit_and_push(
                                        &content_dir,
                                        &message,
                                        &token,
                                        &author_name,
                                        &author_email,
                                    );
                                }
                            }
                        }

                        if let Some(folder) = dropbox_folder {
                            if let Err(e) = dropbox::push(&content_dir, &folder).await {
                                eprintln!("Dropbox auto-push on exit failed: {e}");
                            }
                        }

                        let _ = window.close();
                    });
                }
            });
        }

        Ok(())
    });

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(devtools);
    }

    builder
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_recents,
            commands::create_file,
            commands::update_file,
            commands::delete_file,
            commands::rename_file,
            commands::move_file,
            commands::list_binders,
            commands::create_binder,
            commands::rename_binder,
            commands::delete_binder,
            commands::create_folder,
            commands::list_folders,
            commands::delete_folder,
            commands::check_models,
            commands::get_supported_models,
            commands::download_model,
            commands::load_models,
            commands::run_chat,
            commands::cancel_chat,
            commands::read_file,
            commands::read_files,
            commands::get_file_history,
            commands::create_document_context,
            commands::update_text_chunk,
            commands::get_document_context,
            commands::reindex_notes,
            commands::get_index_status,
            commands::get_config,
            commands::set_default_llm_model,
            commands::set_system_prompt,
            commands::goto_file_version,
            commands::undo_file,
            commands::redo_file,
            commands::github_device_start,
            commands::github_device_poll,
            commands::github_logout,
            commands::github_get_user,
            commands::git_status,
            commands::git_set_repo,
            commands::set_auto_push_on_exit,
            commands::set_ai_enabled,
            commands::set_ai_provider,
            commands::set_openrouter_model,
            commands::set_openrouter_api_key,
            commands::has_openrouter_api_key,
            commands::clear_openrouter_api_key,
            commands::get_openrouter_models,
            commands::git_pull,
            commands::git_commit_and_push,
            commands::export_manuscript,
            commands::search_project,
            commands::replace_in_project,
            commands::import_sources,
            commands::dropbox_start_login,
            commands::dropbox_poll_login,
            commands::dropbox_cancel_login,
            commands::dropbox_logout,
            commands::dropbox_get_account,
            commands::dropbox_set_folder,
            commands::set_dropbox_auto_push_on_exit,
            commands::dropbox_status,
            commands::dropbox_push,
            commands::dropbox_pull,
            commands::set_sync_provider,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
