use super::fs::{self, File};
use super::memory::{MemoryDocumentAnalysisExt, NoteDocument, UserMemoryExt};
use super::prompts::RAG_CHAT_PROMPT;
use super::responses::Response;
use super::workers::{Job, JobStatus};
use super::AppState;
use crate::llm::{download_model_to_cache, ModelStatus, ModelType, SupportedModel};
use crate::memory::{SearchResult, TextChunk};
use crate::utils::{ChatMode, EditAction};
use tauri::State;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/**
 *  FS Commands
 */

#[tauri::command]
pub async fn list_recents(state: State<'_, AppState>) -> Result<Vec<File>, String> {
    let config = state.config.lock().await;
    let content_dir = &config.content_directory;
    let result = fs::get_recent(content_dir).map_err(|e| e.to_string())?;
    Ok(result)
}

/// Splits off the trailing filename and returns the remaining path segments,
/// e.g. `"Novel/Chapter 1/Scene 1.md"` -> `["Novel", "Chapter 1"]`.
fn relative_dir_segments(name: &str) -> Vec<String> {
    name.rsplit_once('/')
        .map(|(dir, _)| dir.split('/').map(String::from).collect())
        .unwrap_or_default()
}

#[tauri::command]
pub async fn create_file(
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<File, String> {
    let config = state.config.lock().await;
    let path = config.content_directory.join(&name);
    let relative_dir = relative_dir_segments(&name);
    let result = fs::create_file(&path, &content, relative_dir).map_err(|e| e.to_string())?;

    let mut undo_tree = state.undotree.lock().await;
    undo_tree.add_change(&name, &content);
    undo_tree
        .save(&config.undotree_dir)
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn update_file(
    name: String,
    content: &str,
    state: State<'_, AppState>,
) -> Result<File, String> {
    let config = state.config.lock().await;
    let path = config.content_directory.join(&name);

    let last_modified = std::fs::metadata(&path).and_then(|m: std::fs::Metadata| m.modified()).ok()
        .and_then(|t: std::time::SystemTime| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
        .map(|d: std::time::Duration| d.as_secs()).unwrap_or(0);
    let folder = name.split_once('/').map(|(dir, _)| dir.to_string());
    let file_to_update = File { path, name, last_modified, folder };

    let result = fs::update_file(&file_to_update, content).map_err(|e| e.to_string());

    let mut undo_tree = state.undotree.lock().await;
    undo_tree.add_change(&file_to_update.name, content);
    undo_tree
        .save(&config.undotree_dir)
        .map_err(|e| e.to_string())
        .unwrap_or_else(|e| eprintln!("Failed to save undo tree: {}", e));
    result
}

#[tauri::command]
pub async fn delete_file(name: String, state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().await;
    let path = config.content_directory.join(&name);

    let last_modified = std::fs::metadata(&path).and_then(|m: std::fs::Metadata| m.modified()).ok()
        .and_then(|t: std::time::SystemTime| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
        .map(|d: std::time::Duration| d.as_secs()).unwrap_or(0);
    let folder = name.split_once('/').map(|(dir, _)| dir.to_string());
    let file_to_delete = File { path, name, last_modified, folder };

    fs::delete_file(&file_to_delete).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_file(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<File, String> {
    let config = state.config.lock().await;

    let old_path = config.content_directory.join(&old_name);
    let result = fs::rename_file(&old_path, &new_name).map_err(|e| e.to_string())?;

    let mut undo_tree = state.undotree.lock().await;
    undo_tree.rename_entry(&old_name, &new_name);
    undo_tree
        .save(&config.undotree_dir)
        .map_err(|e| e.to_string())?;

    // Rename in memory database
    let memory = state.memory.lock().await;
    memory.rename_document(&old_name, &new_name).await.map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn read_file(name: String, state: State<'_, AppState>) -> Result<String, String> {
    let undo_tree = state.undotree.lock().await;

    if let Some(history) = undo_tree.get_history(&name) {
        if let Some(current_index) = history.current {
            if let Some(current_node) = history.nodes.get(current_index.0) {
                let content = current_node.content.clone();
                return Ok(content);
            }
        }
    }

    // Drop the lock on the undo_tree before acquiring the config lock
    drop(undo_tree);

    let config = state.config.lock().await;
    let path = config.content_directory.join(&name);
    let last_modified = std::fs::metadata(&path).and_then(|m: std::fs::Metadata| m.modified()).ok()
        .and_then(|t: std::time::SystemTime| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
        .map(|d: std::time::Duration| d.as_secs()).unwrap_or(0);
    let folder = name.split_once('/').map(|(dir, _)| dir.to_string());
    let file = File { path, name, last_modified, folder };
    file.read_content().map_err(|e| e.to_string())
}

/// Batched version of `read_file`: resolves and reads many files in one IPC
/// round-trip, concurrently, instead of one `read_file` call per file.
/// Skips the undo-tree override that `read_file` applies, so entries with
/// unsaved undo history return on-disk content here.
#[tauri::command]
pub async fn read_files(
    names: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<(String, Result<String, String>)>, String> {
    let config = state.config.lock().await;
    let content_dir = config.content_directory.clone();
    drop(config);

    let files: Vec<File> = names
        .into_iter()
        .map(|name| {
            let path = content_dir.join(&name);
            let last_modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let folder = name.split_once('/').map(|(dir, _)| dir.to_string());
            File { path, name, last_modified, folder }
        })
        .collect();

    let results = fs::read_contents_batch(files).await;
    Ok(results
        .into_iter()
        .map(|(file, result)| (file.name, result.map_err(|e| e.to_string())))
        .collect())
}

#[tauri::command]
pub async fn list_binders(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let config = state.config.lock().await;
    fs::list_binders(&config.content_directory).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_binder(name: String, state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().await;
    fs::create_binder(&config.content_directory, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_binder(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config = state.config.lock().await;
    fs::rename_binder(&config.content_directory, &old_name, &new_name).map_err(|e| e.to_string())?;

    // Every writing inside moved with the folder: keep their history and
    // embeddings keyed to the new paths.
    let mut undo_tree = state.undotree.lock().await;
    undo_tree.rename_prefix(&old_name, &new_name);
    undo_tree
        .save(&config.undotree_dir)
        .map_err(|e| e.to_string())?;

    let memory = state.memory.lock().await;
    memory
        .rename_document_prefix(&old_name, &new_name)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Moves a writing to a new content-relative path (any folder, any binder),
/// keeping its version history and embeddings.
#[tauri::command]
pub async fn move_file(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<File, String> {
    let config = state.config.lock().await;
    let result =
        fs::move_file(&config.content_directory, &old_name, &new_name).map_err(|e| e.to_string())?;

    let mut undo_tree = state.undotree.lock().await;
    undo_tree.rename_entry(&old_name, &new_name);
    undo_tree
        .save(&config.undotree_dir)
        .map_err(|e| e.to_string())?;

    let memory = state.memory.lock().await;
    memory.rename_document(&old_name, &new_name).await.map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn delete_binder(name: String, state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().await;
    fs::delete_binder(&config.content_directory, &name).map_err(|e| e.to_string())
}

/// Creates an empty nested folder (e.g. a chapter inside a Novel binder) at
/// `relative_path`, relative to the content directory.
#[tauri::command]
pub async fn create_folder(relative_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().await;
    fs::create_folder(&config.content_directory, &relative_path).map_err(|e| e.to_string())
}

/// Lists every directory (binder, chapter, scene grouping, ...) under the
/// content directory, so folders with no writings yet still show up.
#[tauri::command]
pub async fn list_folders(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let config = state.config.lock().await;
    fs::list_folders(&config.content_directory).map_err(|e| e.to_string())
}

/// Deletes an empty nested folder (chapter, scene grouping, ...). Refuses if
/// it still contains anything, same rule as `delete_binder`.
#[tauri::command]
pub async fn delete_folder(relative_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().await;
    fs::delete_folder(&config.content_directory, &relative_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_file_history(
    name: String,
    state: State<'_, AppState>,
) -> Result<Option<super::undotree::History>, String> {
    let undo_tree = state.undotree.lock().await;
    Ok(undo_tree.get_history(&name).cloned())
}

#[tauri::command]
pub async fn check_models(state: State<'_, AppState>) -> Result<Vec<ModelStatus>, String> {
    let model = state.model.lock().await;
    Ok(model.check_models().await)
}

#[tauri::command]
pub async fn get_supported_models(state: State<'_, AppState>) -> Result<Vec<SupportedModel>, String> {
    let model = state.model.lock().await;
    Ok(model.get_supported_models().await)
}

#[tauri::command]
pub async fn download_model(
    model_id: String,
    handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !state.config.lock().await.ai_enabled {
        return Err("AI is disabled. Enable it in Settings before downloading models.".to_string());
    }
    let cache_dir = super::utils::get_app_dir()
        .map_err(|e| e.to_string())?
        .join("models");
    download_model_to_cache(&model_id, cache_dir, handle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_models(handle: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let model_id = {
        let config = state.config.lock().await;
        if !config.ai_enabled {
            return Err("AI is disabled. Enable it in Settings before loading models.".to_string());
        }
        config
            .default_llm_model_id
            .as_deref()
            .unwrap_or("qwen_2_5_1_5b_instruct")
            .to_string()
    };
    let mut model = state.model.lock().await;
    model
        .download_or_load_model_by_id(&model_id, ModelType::Chat, handle.clone())
        .await
        .map_err(|e| e.to_string())?;
    model
        .download_or_load_model_by_id(&model_id, ModelType::AutoComplete, handle)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Model loaded successfully.".to_string())
}

// TODO! add job type for chat vs autocomplete
#[tauri::command]
pub async fn run_chat(
    message: String,
    mode: ChatMode,
    edit_action: Option<EditAction>,
    state: State<'_, AppState>,
) -> Result<Uuid, String> {
    if !state.config.lock().await.ai_enabled {
        return Err("AI is disabled. Enable it in Settings before using chat.".to_string());
    }

    let job_id = Uuid::new_v4();
    let cancellation_token = CancellationToken::new();

    let job = Job {
        id: job_id,
        edit_action,
        mode,
        message,
        cancellation_token: cancellation_token.clone(),
    };

    state
        .workers
        .cancellation_tokens
        .insert(job_id, cancellation_token);

    state.workers.statuses.insert(job_id, JobStatus::Queued);

    state
        .workers
        .sender
        .send(job)
        .await
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

#[tauri::command]
pub async fn cancel_chat(job_id: Uuid, state: State<'_, AppState>) -> Result<bool, String> {
    if let Some((_, token)) = state.workers.cancellation_tokens.remove(&job_id) {
        token.cancel();
    }
    Ok(true)
}

/**
 *  RAG Commands
 */

#[tauri::command]
pub async fn update_text_chunk(
    id: String,
    correction: String,
    state: State<'_, AppState>,
) -> Result<Response<String>, String> {
    let memory = state.memory.lock().await;
    let text_chunk = memory
        .find_text_chunk_by_id(&id)
        .await
        .map_err(|e| e.to_string())?;
    match text_chunk {
        Some(chunk) => {
            memory.update_dirty_chunk(chunk, &correction).await;
            Ok(Response::success("updated".to_string()))
        }
        None => Err("Text chunk not found".to_string()),
    }
}

#[tauri::command]
pub async fn create_document_context(
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Response<Vec<TextChunk>>, String> {
    let memory = state.memory.lock().await;
    let document = NoteDocument::from_parts(&name, &content);

    let document_context = memory.to_document_context(document).await;
    match document_context {
        Some(document) => {
            let result = memory.get_dirty_document_chunk(document.id.unwrap()).await;
            return Ok(Response::success(result));
        }
        None => {
            return Ok(Response::success(Vec::new()));
        }
    }
}

#[tauri::command]
pub async fn get_document_context(
    name: String,
    state: State<'_, AppState>,
) -> Result<Response<Vec<TextChunk>>, String> {
    let memory = state.memory.lock().await;
    let document = memory.find_document_by_title(&name).await;
    match document {
        Ok(doc) => {
            if let Some(document) = doc {
                let thing_id = document.get_thing_id();
                match thing_id {
                    Some(id) => {
                        let result = memory.get_dirty_document_chunk(id).await;
                        return Ok(Response::success(result));
                    }
                    None => {
                        return Ok(Response::success(Vec::new()));
                    }
                }
            } else {
                return Ok(Response::success(Vec::new()));
            }
        }
        Err(_) => todo!(),
    }
}

#[tauri::command]
pub async fn search_documents(
    query: String,
    state: State<'_, AppState>,
) -> Result<Response<super::responses::SearchResponse>, String> {
    if !state.config.lock().await.ai_enabled {
        return Err("AI is disabled. Enable it in Settings before using chat.".to_string());
    }

    let memory = state.memory.lock().await;
    let search_results = memory
        .search_documents(&query, 2)
        .await
        .map_err(|e| e.to_string())?;

    let job_id = Uuid::new_v4();
    let cancellation_token = CancellationToken::new();

    let context = search_results
        .iter()
        .map(|res| res.content.clone())
        .collect::<Vec<String>>()
        .join("\n---\n");

    let message = format!(
        "{}",
        RAG_CHAT_PROMPT
        .replace("{context}", &context)
        .replace("{query}", &query)
    );

    let job = Job {
        id: job_id,
        edit_action: None,
        mode: ChatMode::RagChat,
        message,
        cancellation_token: cancellation_token.clone(),
    };

    state
        .workers
        .cancellation_tokens
        .insert(job_id, cancellation_token);

    state.workers.statuses.insert(job_id, JobStatus::Queued);

    state
        .workers
        .sender
        .send(job)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Response::success(super::responses::SearchResponse {
        results: search_results,
        job_id,
    }))
}

/**
 *  Config Commands
 */
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<super::config::AppConfig, String> {
    let config = state.config.lock().await;
    Ok(serde_json::from_str(&serde_json::to_string(&*config).unwrap()).unwrap())
}

#[tauri::command]
pub async fn set_default_llm_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config_path = super::utils::get_app_dir()
        .map_err(|e| e.to_string())?
        .join("config.yaml");
    let config_path_str = config_path
        .to_str()
        .ok_or_else(|| "Config path not UTF-8".to_string())?;
    let mut config = state.config.lock().await;
    config.default_llm_model_id = Some(model_id);
    config.save(config_path_str).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_system_prompt(
    prompt: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config_path = super::utils::get_app_dir()
        .map_err(|e| e.to_string())?
        .join("config.yaml");
    let config_path_str = config_path
        .to_str()
        .ok_or_else(|| "Config path not UTF-8".to_string())?;
    let mut config = state.config.lock().await;
    config.system_prompt = if prompt.trim().is_empty() { None } else { Some(prompt) };
    config.save(config_path_str).map_err(|e| e.to_string())?;
    
    // Clear the chat session cache so the new system prompt takes effect immediately
    let session_cache_path = super::utils::get_app_dir()
        .map_err(|e| e.to_string())?
        .join("chat.llama");
    let _ = std::fs::remove_file(session_cache_path);
    
    Ok(())
}

#[tauri::command]
pub async fn clear_chat_session() -> Result<(), String> {
    let session_cache_path = super::utils::get_app_dir()
        .map_err(|e| e.to_string())?
        .join("chat.llama");
    // Ignore errors if the file doesn't exist
    let _ = std::fs::remove_file(session_cache_path);
    Ok(())
}

/**
 *  Memory Commands
 */
#[tauri::command]
pub async fn get_chat_sessions(
    state: State<'_, AppState>,
) -> Result<Option<super::memory::ChatSession>, String> {
    let memory = state.memory.lock().await;
    let sessions = memory
        .get_chat_sessions()
        .await
        .map_err(|e| e.to_string())?;
    Ok(sessions)
}

/**
 *  UNDO TREE Commands
 */

#[tauri::command]
pub async fn goto_file_version(
    name: String,
    node_id: usize,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let config = state.config.lock().await;
    let mut undo_tree = state.undotree.lock().await;

    let result = undo_tree
        .goto_version(&name, node_id)
        .map(|content| content.to_string());

    if let Some(content_string) = &result {
        undo_tree
            .save(&config.undotree_dir)
            .map_err(|e| e.to_string())?;

        Ok(Some(content_string.clone()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn undo_file(name: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let config = state.config.lock().await;
    let mut undo_tree = state.undotree.lock().await;

    let result = undo_tree.undo(&name).map(|s| s.to_string());

    if result.is_some() {
        undo_tree
            .save(&config.undotree_dir)
            .map_err(|e| e.to_string())?;
    }

    Ok(result)
}

#[tauri::command]
pub async fn redo_file(name: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let config = state.config.lock().await;
    let mut undo_tree = state.undotree.lock().await;

    let result = undo_tree.redo_latest_branch(&name).map(|s| s.to_string());

    if result.is_some() {
        undo_tree
            .save(&config.undotree_dir)
            .map_err(|e| e.to_string())?;
    }

    Ok(result)
}

/**
 *  Git / GitHub Commands
 */
#[tauri::command]
pub async fn github_device_start() -> Result<super::git::DeviceCode, String> {
    super::git::start_device_flow().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_device_poll(device_code: String) -> Result<super::git::DevicePollResult, String> {
    super::git::poll_device_flow(&device_code).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_logout() -> Result<(), String> {
    super::git::clear_token().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_get_user() -> Result<Option<super::git::GitHubUser>, String> {
    let token = super::git::load_token().map_err(|e| e.to_string())?;
    match token {
        Some(token) => {
            let user = super::git::fetch_github_user(&token).await.map_err(|e| e.to_string())?;
            Ok(Some(user))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn git_status(state: State<'_, AppState>) -> Result<Vec<super::git::GitFileStatus>, String> {
    let config = state.config.lock().await;
    super::git::status(&config.content_directory).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_set_repo(owner_repo: String, state: State<'_, AppState>) -> Result<(), String> {
    let config_path = super::utils::get_app_dir()
        .map_err(|e| e.to_string())?
        .join("config.yaml");
    let config_path_str = config_path
        .to_str()
        .ok_or_else(|| "Config path not UTF-8".to_string())?;

    let mut config = state.config.lock().await;
    super::git::set_remote(&config.content_directory, &owner_repo).map_err(|e| e.to_string())?;
    config.github_repo = Some(owner_repo);
    config.save(config_path_str).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_auto_push_on_exit(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let config_path = super::utils::get_app_dir()
        .map_err(|e| e.to_string())?
        .join("config.yaml");
    let config_path_str = config_path
        .to_str()
        .ok_or_else(|| "Config path not UTF-8".to_string())?;

    let mut config = state.config.lock().await;
    config.auto_push_on_exit = enabled;
    config.save(config_path_str).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_ai_enabled(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let config_path = super::utils::get_app_dir()
        .map_err(|e| e.to_string())?
        .join("config.yaml");
    let config_path_str = config_path
        .to_str()
        .ok_or_else(|| "Config path not UTF-8".to_string())?;

    let mut config = state.config.lock().await;
    config.ai_enabled = enabled;
    config.save(config_path_str).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn git_pull(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().await;

    if config.github_repo.is_none() {
        return Err("No GitHub repository configured".to_string());
    }

    let token = super::git::load_token()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Not logged in to GitHub".to_string())?;

    super::git::pull(&config.content_directory, &token).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_commit_and_push(message: String, state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().await;

    if config.github_repo.is_none() {
        return Err("No GitHub repository configured".to_string());
    }

    let token = super::git::load_token()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Not logged in to GitHub".to_string())?;

    let user = super::git::fetch_github_user(&token).await.map_err(|e| e.to_string())?;
    let author_name = user.name.unwrap_or_else(|| user.login.clone());
    let author_email = format!("{}@users.noreply.github.com", user.login);

    super::git::commit_and_push(
        &config.content_directory,
        &message,
        &token,
        &author_name,
        &author_email,
    )
    .map_err(|e| e.to_string())
}
