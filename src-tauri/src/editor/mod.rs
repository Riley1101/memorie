//! The seam between the Svelte UI and the Rust document engine.
//!
//! One document open in the app is one [`Editor`] session, keyed by the
//! writing's path. The UI sends commands and gets state back; it never sees a
//! transaction, an operation or a block path, and it cannot mutate the document
//! any other way.
//!
//! Six commands, not sixty:
//!
//! ```text
//! editor_open      parse a writing (or pick up the session already open)
//! editor_open_text open a document from text, for content with no file yet
//! editor_apply     apply commands, get back only what changed
//! editor_state     the current state, without changing anything
//! editor_markdown  what would be written to disk
//! editor_save      write it, through the same path the old save used
//! editor_close     forget the session
//! ```
//!
//! Nothing in the app calls these yet: the Milkdown editor is still the one
//! the writer types into. This is the surface Phase 6 moves it onto, landed
//! separately so the engine can be exercised from the console (and from tests)
//! before anything depends on it.

pub mod wire;

use crate::fs::File;
use crate::AppState;
use editor_core::Change;
use editor_core::{markdown, Editor};
use std::collections::HashMap;
use tauri::State;
use tokio::sync::Mutex;
use wire::{WireCommand, WireState, WireUpdate};

/// The documents open right now, by writing path.
#[derive(Default)]
pub struct Sessions(Mutex<HashMap<String, Editor>>);

fn now_millis() -> Option<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|since| since.as_millis() as i64)
}

/// Reads a writing the way the rest of the app does: the version history wins
/// when it has content for this name (a restored version the file doesn't have
/// yet), otherwise the file on disk. A writing that doesn't exist yet — a draft
/// whose name is decided but nothing written — opens empty.
async fn read_source(state: &AppState, name: &str) -> Result<String, String> {
    if let Some(content) = state.undotree.lock().await.current_content(name) {
        return Ok(content);
    }
    let config = state.config.lock().await;
    let path = config.content_directory.join(name);
    drop(config);
    if !path.exists() {
        return Ok(String::new());
    }
    File::new(path, None)
        .read_content()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn editor_open(
    name: String,
    #[allow(unused_variables)] reload: Option<bool>,
    state: State<'_, AppState>,
) -> Result<WireState, String> {
    let reload = reload.unwrap_or(false);
    {
        let sessions = state.editors.0.lock().await;
        // Opening a document that is already open must not throw away edits
        // the writer hasn't saved; `reload` is the explicit way to do that.
        if let Some(editor) = sessions.get(&name) {
            if !reload {
                return Ok(wire::state_view(&name, editor));
            }
        }
    }

    let source = read_source(&state, &name).await?;
    let editor = Editor::new(markdown::parse(name.as_str(), &source));
    let view = wire::state_view(&name, &editor);
    state.editors.0.lock().await.insert(name, editor);
    Ok(view)
}

/// Opens a document from text rather than from disk, replacing any session
/// under that name. For content that has no file yet — an import preview, a
/// benchmark, a test — and deliberately never writes anything: `editor_save`
/// would write it to `name` like any other writing.
#[tauri::command]
pub async fn editor_open_text(
    name: String,
    text: String,
    state: State<'_, AppState>,
) -> Result<WireState, String> {
    let editor = Editor::new(markdown::parse(name.as_str(), &text));
    let view = wire::state_view(&name, &editor);
    state.editors.0.lock().await.insert(name, editor);
    Ok(view)
}

#[tauri::command]
pub async fn editor_apply(
    name: String,
    commands: Vec<WireCommand>,
    state: State<'_, AppState>,
) -> Result<WireUpdate, String> {
    let mut sessions = state.editors.0.lock().await;
    let editor = sessions
        .get_mut(&name)
        .ok_or_else(|| format!("'{name}' is not open"))?;

    let started = std::time::Instant::now();
    let at = now_millis();
    let mut change = Change::default();
    for command in commands {
        // Offsets are converted against the document as it is now, so a batch
        // of commands means the same thing as the same commands sent one by one.
        for command in command.into_commands(editor.document())? {
            change.merge(editor.apply_at(command, at).map_err(|e| e.to_string())?);
        }
    }
    // The whole batch is one update: what it touched, not what the document is.
    // The timing covers the engine and the update it builds — not the bridge,
    // which is what the frontend measures around this call.
    let micros = started.elapsed().as_micros() as u64;
    Ok(wire::update_view(editor, &change, micros))
}

#[tauri::command]
pub async fn editor_state(name: String, state: State<'_, AppState>) -> Result<WireState, String> {
    let sessions = state.editors.0.lock().await;
    let editor = sessions
        .get(&name)
        .ok_or_else(|| format!("'{name}' is not open"))?;
    Ok(wire::state_view(&name, editor))
}

#[tauri::command]
pub async fn editor_markdown(name: String, state: State<'_, AppState>) -> Result<String, String> {
    let sessions = state.editors.0.lock().await;
    let editor = sessions
        .get(&name)
        .ok_or_else(|| format!("'{name}' is not open"))?;
    Ok(markdown::to_markdown(editor.document()))
}

#[tauri::command]
pub async fn editor_save(name: String, state: State<'_, AppState>) -> Result<File, String> {
    let markdown = {
        let sessions = state.editors.0.lock().await;
        let editor = sessions
            .get(&name)
            .ok_or_else(|| format!("'{name}' is not open"))?;
        markdown::to_markdown(editor.document())
    };
    // The same path `update_file` takes, so the version history, the file
    // watcher and the indexer see a save from here exactly as before.
    crate::commands::save_writing(&state, &name, &markdown).await
}

#[tauri::command]
pub async fn editor_close(name: String, state: State<'_, AppState>) -> Result<(), String> {
    state.editors.0.lock().await.remove(&name);
    Ok(())
}
