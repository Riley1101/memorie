use super::fs::{self, File};
use super::memory::{EmbeddingDocument, MemoryDocumentContextExt, NoteDocument, UserMemory};
use super::responses::Response;
use super::workers::{Job, JobStatus};
use super::AppState;
use crate::error::FileError;
use crate::llm::ModelType;
use crate::utils::ChatMode;
use ammonia::is_html;
use htmd::HtmlToMarkdown;
use tauri::State;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/**
 *  FS Commands
 */
#[tauri::command]
pub async fn list_files(state: State<'_, AppState>) -> Result<Vec<File>, String> {
    let config = state.config.lock().await;
    let content_dir = &config.content_directory;
    let result = fs::discover_files(content_dir).map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub async fn list_recents(state: State<'_, AppState>) -> Result<Vec<File>, String> {
    let config = state.config.lock().await;
    let content_dir = &config.content_directory;
    let result = fs::get_recent(content_dir).map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub async fn create_file(
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<File, String> {
    let config = state.config.lock().await;
    let path = config.content_directory.join(&name);
    let result = fs::create_file(&path, &content).map_err(|e| e.to_string())?;

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

    let file_to_update = File { path, name };

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

    let file_to_delete = File { path, name };

    fs::delete_file(&file_to_delete).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_file(name: String, state: State<'_, AppState>) -> Result<String, String> {
    let undo_tree = state.undotree.lock().await;

    if let Some(history) = undo_tree.get_history(&name) {
        if let Some(current_index) = history.current {
            if let Some(current_node) = history.nodes.get(current_index.0) {
                let content = current_node.content.clone();

                let is_html_content = is_html(&content);

                if is_html_content {
                    let converter = HtmlToMarkdown::builder()
                        .skip_tags(vec!["script", "style"])
                        .build();
                    let result = converter
                        .convert(&content)
                        .map_err(|e| FileError::ContentConversionError(e.to_string()));
                    if let Ok(markdown_content) = result {
                        return Ok(markdown_content);
                    }
                } else {
                    return Ok(content);
                }
            }
        }
    }

    // Drop the lock on the undo_tree before acquiring the config lock
    drop(undo_tree);

    let config = state.config.lock().await;
    let path = config.content_directory.join(&name);
    let file = File { path, name };
    file.read_content().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_file_history(
    name: String,
    state: State<'_, AppState>,
) -> Result<Option<super::undotree::History>, String> {
    let undo_tree = state.undotree.lock().await;
    Ok(undo_tree.get_history(&name).cloned())
}

/**
 *  LLM Commands
 */
#[tauri::command]
pub async fn load_models(state: State<'_, AppState>) -> Result<String, String> {
    let mut model = state.model.lock().await;
    model
        .download_or_load_model(ModelType::Chat)
        .await
        .map_err(|e| e.to_string())?;
    model
        .download_or_load_model(ModelType::AutoComplete)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Model loaded successfully.".to_string())
}

// TODO! add job type for chat vs autocomplete
#[tauri::command]
pub async fn run_chat(
    message: String,
    mode: ChatMode,
    state: State<'_, AppState>,
) -> Result<Uuid, String> {
    let job_id = Uuid::new_v4();
    let cancellation_token = CancellationToken::new();

    let job = Job {
        id: job_id,
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

#[tauri::command]
pub async fn get_chat_status(
    job_id: Uuid,
    state: State<'_, AppState>,
) -> Result<JobStatus, String> {
    if let Some(status) = state.workers.statuses.get(&job_id) {
        Ok(status.clone())
    } else {
        Err("Job not found".to_string())
    }
}

/**
 *  RAG Commands
 */

#[tauri::command]
pub async fn create_document_context(
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Response<String>, String> {
    let memory = state.memory.lock().await;
    let document = NoteDocument::from_parts(&name, &content);
    println!("Creating document context for: {}", name);

    let document_context = memory.to_document_context(document).await;

    println!(
        "Document context created: {:?}",
        document_context.embeddings()
    );

    Ok(Response::success("Created Embeddings".to_string()))
}

#[tauri::command]
pub async fn search_documents(
    _query: String,
    _state: State<'_, AppState>,
) -> Result<Response<Vec<EmbeddingDocument>>, String> {
    let vec = Vec::new();
    let response = Response::success(vec);
    Ok(response)
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
pub async fn undo_file(name: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let config = state.config.lock().await;
    let mut undo_tree = state.undotree.lock().await;
    if let Some(previous_content) = undo_tree.undo(&name) {
        undo_tree
            .save(&config.undotree_dir)
            .map_err(|e| e.to_string())?;
        Ok(Some(previous_content))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn redo_file(name: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let config = state.config.lock().await;
    let mut undo_tree = state.undotree.lock().await;
    if let Some(next_content) = undo_tree.redo(&name) {
        undo_tree
            .save(&config.undotree_dir)
            .map_err(|e| e.to_string())?;
        Ok(Some(next_content))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn goto_file_version(
    name: String,
    node_id: usize,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let config = state.config.lock().await;
    let mut undo_tree = state.undotree.lock().await;
    if let Some(content) = undo_tree.goto_version(&name, node_id) {
        undo_tree
            .save(&config.undotree_dir)
            .map_err(|e| e.to_string())?;
        Ok(Some(content))
    } else {
        Ok(None)
    }
}
