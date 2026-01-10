use super::fs::{self, File};
use super::memory::{MemoryDocumentAnalysisExt, NoteDocument, UserMemoryExt};
use super::responses::Response;
use super::workers::{Job, JobStatus};
use super::AppState;
use crate::llm::ModelType;
use crate::memory::TextChunk;
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
                return Ok(content);
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
    edit_action: Option<EditAction>,
    state: State<'_, AppState>,
) -> Result<Uuid, String> {
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
                        println!("Fetching document chunks for document ID: {}", id);
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
    _query: String,
    _state: State<'_, AppState>,
) -> Result<Response<Vec<String>>, String> {
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
