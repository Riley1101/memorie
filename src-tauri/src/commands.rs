use super::events::{ChatEvents, ChatStreamInProgress};
use futures::StreamExt;
use kalosm::language::Document;
use tauri::State;
use tauri::{AppHandle, Emitter, Window};

use crate::fs::{self, File};
use crate::AppState;

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
pub async fn load_model(state: State<'_, AppState>) -> Result<String, String> {
    let mut model = state.model.lock().await;
    let llama = model.load_model().await.map_err(|e| e.to_string())?;
    model.set_loaded_model(llama);
    Ok("Model loaded successfully.".to_string())
}

#[tauri::command]
pub async fn run_chat(
    window: Window,
    message: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let model = state.model.lock().await;

    let mut chat_session = model.run_chat().await.map_err(|e| e.to_string())?;

    let mut stream = chat_session.add_message(message);

    while let Some(token) = stream.next().await {
        println!("{:?}", token);
        window
            .emit(
                ChatEvents::InProgress.as_str(),
                ChatStreamInProgress { content: &token },
            )
            .unwrap();
    }

    Ok(ChatEvents::Completed.as_str().to_string())
}

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
