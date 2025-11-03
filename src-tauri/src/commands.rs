use kalosm::language::Document;
use tauri::State;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::fs::{self, File};
use super::memory::UserMemory;
use super::workers::{Job, JobStatus};
use super::AppState;

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
    content: &str,
    state: State<'_, AppState>,
) -> Result<File, String> {
    let config = state.config.lock().await;
    let path = config.content_directory.join(&name);
    let result = fs::create_file(&path, content).map_err(|e| e.to_string())?;

    let mut undo_tree = state.undotree.lock().await;

    undo_tree.add_change(&name, content);
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
pub async fn load_model(state: State<'_, AppState>) -> Result<String, String> {
    let mut model = state.model.lock().await;
    let llama = model.load_model().await.map_err(|e| e.to_string())?;
    model.set_loaded_model(llama);
    Ok("Model loaded successfully.".to_string())
}
#[tauri::command]
pub async fn run_chat(message: String, state: State<'_, AppState>) -> Result<Uuid, String> {
    let job_id = Uuid::new_v4();
    let cancellation_token = CancellationToken::new();
    let memory = state.memory.lock().await;

    memory
        .save_chat_session(&job_id.to_string())
        .await
        .map_err(|e| e.to_string())?;

    let job = Job {
        id: job_id,
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
pub async fn create_embeddings(
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let memory = state.memory.lock().await;

    let table = &memory.document_table;
    let document = Document::from_parts(name, content);
    table.insert(document).await.map_err(|e| e.to_string())?;

    Ok("Hello".to_string())
}

#[tauri::command]
pub async fn search_embeddings(
    query: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let memory = state.memory.lock().await;

    let table = &memory.document_table;
    let context = table
        .search(&query)
        .with_results(1)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|document| {
            format!(
                "Title: {}\nBody: {}\n",
                document.record.title(),
                document.record.body()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(context)
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
pub async fn undo_file(
    name: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
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
