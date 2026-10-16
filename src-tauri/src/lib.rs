mod chunker;
mod codex;
mod commands;
mod config;
mod dropbox;
mod error;
mod export;

mod fs;
mod git;
mod gpu;
mod import;
mod indexer;
mod llm;
mod memory;
mod pdf;
mod prompts;
mod providers;
mod reranker;
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
    indexer: indexer::Indexer,
    workers: LlmEventService,
}

/// How long the exit path waits for the editor to write out unsaved text.
/// Long enough for a large document, short enough that a wedged webview can
/// never hold the window open.
const FLUSH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Asks the frontend to write out anything still sitting in the autosave
/// debounce, and waits for it to confirm. Without this, quitting inside the
/// debounce window loses the last few seconds of typing — and the auto-push
/// below would then commit the stale file.
async fn flush_unsaved(app_handle: &tauri::AppHandle) {
    use tauri::{Emitter, Listener};

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let tx = std::sync::Mutex::new(Some(tx));
    let handler = app_handle.once("memoire://flushed", move |_| {
        if let Ok(mut slot) = tx.lock() {
            if let Some(tx) = slot.take() {
                let _ = tx.send(());
            }
        }
    });

    if app_handle.emit("memoire://flush-before-exit", ()).is_err() {
        app_handle.unlisten(handler);
        return;
    }

    if tokio::time::timeout(FLUSH_TIMEOUT, rx).await.is_err() {
        eprintln!("Editor did not confirm its last save before exit; continuing.");
        app_handle.unlisten(handler);
    }
}

/// Pushes writing to the preferred remote when auto-push on exit is turned on.
/// Runs when the main window closes, and before an update is installed.
async fn run_exit_tasks(app_handle: &tauri::AppHandle) {
    flush_unsaved(app_handle).await;

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
                let author_name =
                    user.name.unwrap_or_else(|| user.login.clone());
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
}

/// Runs the on-exit sync ahead of installing an update. The frontend calls this
/// before `install()` because on Windows the installer quits the app at once,
/// skipping the window-close handler.
#[tauri::command]
async fn prepare_for_update(app_handle: tauri::AppHandle) {
    run_exit_tasks(&app_handle).await;
}

/// Relaunches the app so a freshly installed update takes effect.
#[tauri::command]
fn restart_app(app_handle: tauri::AppHandle) {
    app_handle.restart();
}

/// Everything `setup` needs that can fail before there is a window to report
/// the failure in.
struct Startup {
    config: AppConfig,
    undo_tree: UndoTree,
    model: Model,
    memory: Memory,
    manifest: std::path::PathBuf,
}

/// Loads the config, version history and note database. Returns a message fit
/// to show the user rather than panicking: a corrupt config or database used
/// to kill the process with no window and no explanation.
async fn load_startup() -> Result<Startup, String> {
    let config_path = utils::get_app_dir()
        .map_err(|e| format!("Could not find your application data folder: {e}"))?
        .join("config.yaml");

    let config_path_str = config_path
        .to_str()
        .ok_or_else(|| format!("The config path {} is not valid UTF-8.", config_path.display()))?;

    config::AppConfig::init_default_config(config_path_str)
        .map_err(|e| format!("Could not create {}: {e}", config_path.display()))?;

    let config = config::AppConfig::load(config_path_str)
        .map_err(|e| format!("Could not read {}: {e}", config_path.display()))?;

    let memory = Memory::new()
        .await
        .map_err(|e| format!("Could not open the note database: {e}"))?;

    // History is the one piece we can do without: carry on, saying so.
    let undo_tree = UndoTree::load(&config.undotree_dir).unwrap_or_else(|e| {
        eprintln!(
            "Could not open version history at {}: {e}. History will not be kept this session.",
            config.undotree_dir.display()
        );
        UndoTree::new()
    });

    let manifest = utils::get_app_dir()
        .map_err(|e| format!("Could not find your application data folder: {e}"))?
        .join("db/index-manifest.json");

    let model = Model::new(config.default_llm_model.clone());

    Ok(Startup {
        config,
        undo_tree,
        model,
        memory,
        manifest,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let startup = load_startup().await;

    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();

    let watch_dir = startup
        .as_ref()
        .ok()
        .map(|s| s.config.content_directory.clone());

    let mut startup = Some(startup);

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let startup = match startup.take().expect("setup runs once") {
                Ok(startup) => startup,
                Err(message) => {
                    // The dialog plugin is up by now, so the writer gets a
                    // reason instead of an icon that bounces and vanishes.
                    eprintln!("Startup failed: {message}");

                    // No state was managed, so the window has nothing to talk
                    // to; close it rather than show an empty frame.
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.close();
                    }

                    // `blocking_show` freezes whatever thread it is on, and
                    // setup runs on the main one, so the dialog gets its own.
                    let handle = app.handle().clone();
                    std::thread::spawn(move || {
                        use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                        handle
                            .dialog()
                            .message(format!("{message}\n\nMemoire cannot start."))
                            .kind(MessageDialogKind::Error)
                            .title("Memoire could not start")
                            .blocking_show();
                        std::process::exit(1);
                    });
                    return Ok(());
                }
            };

            let app_handle = app.handle().clone();
            let app_state = AppState {
                config: Mutex::new(startup.config),
                undotree: Mutex::new(startup.undo_tree),
                model: Mutex::new(startup.model),
                memory: startup.memory,
                indexer: indexer::Indexer::new(startup.manifest),
                workers: LlmEventService::new(app_handle),
            };
            app.manage(app_state);

            // Load (and on first run download) the embedding model and reranker in the
            // background, so neither delays the window nor the first notes search.
            // Nothing else holds the config lock yet during setup.
            let ai_enabled = app
                .state::<AppState>()
                .config
                .try_lock()
                .map(|config| config.ai_enabled)
                .unwrap_or(false);
            if ai_enabled {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let memory = &handle.state::<AppState>().memory;
                    if let Err(e) = memory.embedder().await {
                        eprintln!("Could not load the embedding model: {e}");
                    }
                    memory.reranker().await;
                });
            }

            // Keeps the note index current when notes change on disk. The frontend
            // asks for a first pass on startup.
            if let Some(watch_dir) = watch_dir.clone() {
                if let Err(e) = app
                    .state::<AppState>()
                    .indexer
                    .watch(app.handle(), &watch_dir)
                {
                    eprintln!("Could not watch {} for changes: {e}", watch_dir.display());
                }
            }

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
                            // The push can take seconds on a slow network; let
                            // the window say so instead of just sitting there.
                            use tauri::Emitter;
                            let _ = app_handle.emit("memoire://closing", ());

                            run_exit_tasks(&app_handle).await;

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
            commands::read_compile_settings,
            commands::write_compile_settings,
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
            commands::codex_list,
            commands::codex_save,
            commands::codex_delete,
            commands::codex_detect,
            commands::codex_matrix,
            commands::codex_draft,
            prepare_for_update,
            restart_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
