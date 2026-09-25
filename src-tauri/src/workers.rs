use crate::memory::{pretty_title, MemoryDocumentAnalysisExt, SearchResult};
use crate::utils::{ChatContext, ChatTurn, EditAction};

use super::prompts::{
    DOCUMENT_CHAT_PROMPT, EDIT_ACTION_BASE_PROMPT, GRAMMAR_CHECK_PROMPT, INSERTION_GUARDRAILS,
    INTENT_CLASSIFICATION_PROMPT, NORMAL_CHAT_PROMPT, RAG_CHAT_PROMPT, TEXT_COMPLETION,
};
use super::providers::{self, ProviderKind};
use super::responses::{AutoCompleteResponse, GrammarCheckResponse, IntentResponse};
use super::utils::ChatMode;
use dashmap::DashMap;
use futures::StreamExt;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Represents a job to be processed by the LLM event service.
/// Each job has a unique ID, a message to process, and a cancellation token.
pub struct Job {
    pub id: Uuid,
    pub edit_action: Option<EditAction>,
    pub mode: ChatMode,
    pub message: String,
    pub context: ChatContext,
    pub cancellation_token: CancellationToken,
}

/// Service to manage LLM events, including job queuing, status tracking, and cancellation.
/// It uses a bounded channel to queue jobs and a semaphore to limit concurrent processing.
/// It also maintains maps for job statuses and cancellation tokens.
pub struct LlmEventService {
    pub sender: mpsc::Sender<Job>,
    pub statuses: Arc<DashMap<Uuid, JobStatus>>,
    pub cancellation_tokens: Arc<DashMap<Uuid, CancellationToken>>,
}

const MAX_CONCURRENT_LLM_CALLS: usize = 3;

/// Chunks pulled from the index per notes search, before relevance filtering.
const SEARCH_CHUNK_LIMIT: usize = 8;
/// Distinct notes listed as sources under a reply.
const MAX_SOURCES: usize = 3;
/// Prior messages carried into each reply, and how much of each.
const HISTORY_TURNS: usize = 6;
const HISTORY_TURN_CHARS: usize = 1_200;
/// The router only needs enough conversation to resolve "it"/"she"/"that one".
const CLASSIFIER_HISTORY_TURNS: usize = 4;
const CLASSIFIER_TURN_CHARS: usize = 300;
/// Keeps a long open document from blowing past small local models' context windows.
const MAX_DOCUMENT_CHARS: usize = 16_000;

impl LlmEventService {
    /// Creates a new LLM event service.
    /// Initializes the job queue, status map, and cancellation token map.
    /// Spawns a background task to process jobs from the queue with concurrency control.
    /// # Arguments
    /// * `app_handle` - A handle to the Tauri application for emitting events.
    /// # Returns
    /// A new instance of `LlmEventService`.
    pub fn new(app_handle: AppHandle) -> Self {
        let (sender, mut receiver) = mpsc::channel::<Job>(100);
        let statuses = Arc::new(DashMap::new());
        let cancellation_tokens = Arc::new(DashMap::new());

        let statuses_clone = statuses.clone();
        let cancellation_tokens_clone = cancellation_tokens.clone();

        tokio::spawn(async move {
            let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_LLM_CALLS));

            while let Some(job) = receiver.recv().await {
                let statuses_clone = statuses_clone.clone();
                let cancellation_tokens_clone = cancellation_tokens_clone.clone();
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let app_handle = app_handle.clone();

                tokio::spawn(async move {
                    statuses_clone.insert(job.id, JobStatus::Running);
                    let res = tokio::select! {
                        _ = job.cancellation_token.cancelled() => {
                            statuses_clone.insert(job.id, JobStatus::Cancelled);
                            Err("Cancelled".to_string())
                        }
                        res = run_chat_worker(
                            app_handle.clone(),
                            job.message.clone(),
                            job.context.clone(),
                            job.cancellation_token.clone(),
                            job.mode.clone(),
                            job.edit_action.clone(),
                            ) => {
                            res
                        }
                    };

                    match res {
                        Ok(result) => {
                            if !job.cancellation_token.is_cancelled() {
                                statuses_clone.insert(job.id, JobStatus::Completed(result));
                            }
                        }
                        Err(e) => {
                            if !job.cancellation_token.is_cancelled() {
                                statuses_clone.insert(job.id, JobStatus::Failed(e.to_string()));
                                let _ = app_handle.emit(
                                    ChatEvents::Error.as_str(),
                                    serde_json::json!({ "message": e }),
                                );
                            }
                        }
                    }
                    cancellation_tokens_clone.remove(&job.id);
                    drop(permit);
                });
            }
        });

        Self {
            sender,
            statuses,
            cancellation_tokens,
        }
    }
}

/// Worker function to process a chat message using the LLM model.
/// It streams the response back to the application and handles cancellation.
/// # Arguments
/// * `app_handle` - A handle to the Tauri application for emitting events.
/// * `message` - The chat message to process.
/// * `context` - Recent history and the open document, for `ChatMode::Normal`.
/// * `cancellation_token` - A token to signal cancellation of the chat processing.
/// # Returns
/// A `Result` containing the final response string or an error message.
async fn run_chat_worker(
    app_handle: AppHandle,
    message: String,
    context: ChatContext,
    cancellation_token: CancellationToken,
    mode: ChatMode,
    edit_action: Option<EditAction>,
) -> Result<String, String> {
    let state: tauri::State<crate::AppState> = app_handle.state();

    let mut model = state.model.lock().await;

    let response = String::new();

    if mode == ChatMode::Normal {
        let (sys_prompt, model_id, provider, openrouter_model) = {
            let config = state.config.lock().await;
            (
                config.system_prompt.clone().unwrap_or_default(),
                config.default_llm_model_id.clone().unwrap_or_else(|| "qwen_2_5_1_5b_instruct".to_string()),
                config.provider.clone(),
                config.openrouter_model.clone(),
            )
        };
        let sys_prompt = if sys_prompt.trim().is_empty() {
            NORMAL_CHAT_PROMPT.to_string()
        } else {
            sys_prompt
        };

        app_handle
            .emit(ChatEvents::Init.as_str(), "")
            .map_err(|e| e.to_string())?;

        let document = context
            .document_content
            .as_deref()
            .map(str::trim)
            .filter(|content| !content.is_empty())
            .map(|content| {
                (
                    context.document_title.clone().unwrap_or_else(|| "Untitled".to_string()),
                    content.to_string(),
                )
            });
        let document_title = document.as_ref().map(|(title, _)| title.as_str());
        let history = recent_history(&context.history);

        let has_index = state
            .memory
            .index_stats()
            .await
            .map(|s| s.passages > 0)
            .unwrap_or(false);

        let route = route_message(
            &mut model,
            &model_id,
            &provider,
            &openrouter_model,
            &message,
            document_title,
            has_index,
            &history,
            &app_handle,
        )
        .await;

        // Searches stay inside the open note's binder unless asked otherwise.
        let binder = document_title
            .filter(|_| !route.all_notes)
            .and_then(binder_of);

        app_handle
            .emit(
                ChatEvents::Route.as_str(),
                serde_json::json!({
                    "intent": route.intent.as_str(),
                    "query": &route.query,
                    "documentTitle": document_title,
                    "binder": binder,
                }),
            )
            .ok();

        let user_message = match route.intent {
            Intent::SearchNotes => {
                let results = {
                    let memory = &state.memory;
                    memory
                        .search_documents(&route.query, SEARCH_CHUNK_LIMIT, binder)
                        .await
                        .map_err(|e| e.to_string())?
                };
                app_handle
                    .emit(
                        ChatEvents::Sources.as_str(),
                        serde_json::json!({ "results": source_refs(&results) }),
                    )
                    .ok();
                build_rag_message(&results, &route.question)
            }
            Intent::CurrentDocument => match &document {
                Some((title, content)) => build_document_message(title, content, &route.question),
                None => route.question.clone(),
            },
            Intent::General => route.question.clone(),
        };

        match provider {
            ProviderKind::Local => {
                let mut chat_session = model.run_chat(&model_id, &sys_prompt, app_handle.clone()).await.map_err(|e| e.to_string())?;

                // TODO! Add sampler
                let mut stream = chat_session.add_message(with_transcript(&history, &user_message));

                loop {
                    tokio::select! {
                        _ = cancellation_token.cancelled() => {
                            return Err("Chat cancelled".to_string());
                        }
                        token = stream.next() => {
                            match token {
                                Some(token) => {
                                    app_handle
                                        .emit(
                                            ChatEvents::InProgress.as_str(),
                                            ChatStreamInProgress { content: &token },
                                        )
                                        .unwrap();
                                }
                                None => {
                                    app_handle.emit(ChatEvents::Completed.as_str(), "").unwrap();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            ProviderKind::OpenRouter => {
                stream_openrouter(
                    &openrouter_model,
                    &sys_prompt,
                    history,
                    user_message,
                    &cancellation_token,
                    &app_handle,
                    ChatEvents::InProgress.as_str(),
                    ChatEvents::Completed.as_str(),
                )
                .await?;
            }
        }
    } else if mode == ChatMode::EditAction {
        let edit_action = if let Some(action) = edit_action {
            action
        } else {
            EditAction::CorrectGrammar
        };

        let prompt = if edit_action == EditAction::PromptExpansion {
            format!("{}\n {}\n", INSERTION_GUARDRAILS, message)
        } else {
            format!(
                "{}\n{}\n{}",
                EDIT_ACTION_BASE_PROMPT,
                edit_action.into_prompt(),
                message
            )
        };
        let prompt = prompt.lines().map(|s| s.trim()).collect::<Vec<_>>().join("\n");

        let (model_id, provider, openrouter_model) = {
            let config = state.config.lock().await;
            (
                config.default_llm_model_id.clone().unwrap_or_else(|| "qwen_2_5_1_5b_instruct".to_string()),
                config.provider.clone(),
                config.openrouter_model.clone(),
            )
        };

        app_handle
            .emit(ChatEvents::EditActionStart.as_str(), "")
            .map_err(|e| e.to_string())?;

        match provider {
            ProviderKind::Local => {
                let mut chat_session = model.run_chat(&model_id, "", app_handle.clone()).await.map_err(|e| e.to_string())?;

                let mut stream = chat_session.add_message(prompt);

                loop {
                    tokio::select! {
                        _ = cancellation_token.cancelled() => {
                            return Err("Chat cancelled".to_string());
                        }
                        token = stream.next() => {
                            match token {
                                Some(token) => {
                                    app_handle
                                        .emit(
                                            ChatEvents::EditActionInProgress.as_str(),
                                            ChatStreamInProgress { content: &token },
                                        )
                                        .unwrap();
                                }
                                None => {
                                    app_handle.emit(ChatEvents::EditActionCompleted.as_str(), "").unwrap();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            ProviderKind::OpenRouter => {
                stream_openrouter(
                    &openrouter_model,
                    "",
                    Vec::new(),
                    prompt,
                    &cancellation_token,
                    &app_handle,
                    ChatEvents::EditActionInProgress.as_str(),
                    ChatEvents::EditActionCompleted.as_str(),
                )
                .await?;
            }
        }
    } else if mode == ChatMode::Grammar {
        let (model_id, provider, openrouter_model) = {
            let config = state.config.lock().await;
            (
                config.default_llm_model_id.clone().unwrap_or_else(|| "qwen_2_5_1_5b_instruct".to_string()),
                config.provider.clone(),
                config.openrouter_model.clone(),
            )
        };

        let result = match provider {
            ProviderKind::Local => {
                let grammar_check_session = model.run_grammar_check(&model_id, app_handle.clone()).await.map_err(|e| e.to_string())?;

                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        return Err("Grammar check cancelled".to_string());
                    }
                    res = grammar_check_session(&message) => {
                        res.map_err(|e| e.to_string())?
                    }
                }
            }
            ProviderKind::OpenRouter => {
                let api_key = openrouter_api_key()?;
                let model_name = openrouter_model.unwrap_or_else(|| providers::DEFAULT_OPENROUTER_MODEL.to_string());
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        return Err("Grammar check cancelled".to_string());
                    }
                    res = providers::chat_completion_json::<GrammarCheckResponse>(&api_key, &model_name, GRAMMAR_CHECK_PROMPT, &message) => {
                        res.map_err(|e| e.to_string())?
                    }
                }
            }
        };

        app_handle
            .emit(
                ChatEvents::GrammarCheck.as_str(),
                GrammarCheckStreamInProgress {
                    corrections: &result.corrections,
                    explanation: &result.explanation,
                },
            )
            .unwrap();
        app_handle.emit(ChatEvents::Completed.as_str(), "").unwrap();
    } else {
        let (model_id, provider, openrouter_model) = {
            let config = state.config.lock().await;
            (
                config.default_llm_model_id.clone().unwrap_or_else(|| "qwen_2_5_1_5b_instruct".to_string()),
                config.provider.clone(),
                config.openrouter_model.clone(),
            )
        };

        let result = match provider {
            ProviderKind::Local => {
                let chat_session = model.run_autocomplete(&model_id, app_handle.clone()).await.map_err(|e| e.to_string())?;
                let stream = chat_session(&message);
                stream.await.map_err(|e| e.to_string())?
            }
            ProviderKind::OpenRouter => {
                let api_key = openrouter_api_key()?;
                let model_name = openrouter_model.unwrap_or_else(|| providers::DEFAULT_OPENROUTER_MODEL.to_string());
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        return Err("Autocomplete cancelled".to_string());
                    }
                    res = providers::chat_completion_json::<AutoCompleteResponse>(&api_key, &model_name, TEXT_COMPLETION, &message) => {
                        res.map_err(|e| e.to_string())?
                    }
                }
            }
        };
        app_handle
            .emit(
                ChatEvents::AutoComplete.as_str(),
                AutoCompleteStreamInProgress {
                    response: result.clone(),
                },
            )
            .unwrap();
        app_handle.emit(ChatEvents::Completed.as_str(), "").unwrap();
    }

    Ok(response)
}

// -------------------------------------------------------
//  MESSAGE ROUTING
// -------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Intent {
    SearchNotes,
    CurrentDocument,
    General,
}

impl Intent {
    fn as_str(self) -> &'static str {
        match self {
            Intent::SearchNotes => "search_notes",
            Intent::CurrentDocument => "current_document",
            Intent::General => "general",
        }
    }

    fn parse(value: &str) -> Self {
        match value.trim() {
            "search_notes" => Intent::SearchNotes,
            "current_document" => Intent::CurrentDocument,
            _ => Intent::General,
        }
    }
}

struct Route {
    intent: Intent,
    /// Search every note even when a binder's note is open (`/search-all`).
    all_notes: bool,
    /// Standalone phrase to embed for a notes search; empty for other intents.
    query: String,
    /// The user's message with any `/search`-style command stripped.
    question: String,
}

/// Decides how to answer a message. Explicit `/search`, `/doc` and `/chat` commands and
/// obvious small talk skip the model; everything else goes through the classifier.
/// Never fails: a broken classification falls back to a plain chat reply.
#[allow(clippy::too_many_arguments)]
async fn route_message(
    model: &mut crate::llm::Model,
    model_id: &str,
    provider: &ProviderKind,
    openrouter_model: &Option<String>,
    message: &str,
    document_title: Option<&str>,
    has_index: bool,
    history: &[ChatTurn],
    app_handle: &AppHandle,
) -> Route {
    let message = message.trim();
    let has_document = document_title.is_some();

    for (command, intent) in [
        ("/search", Intent::SearchNotes),
        ("/search-all", Intent::SearchNotes),
        ("/doc", Intent::CurrentDocument),
        ("/chat", Intent::General),
    ] {
        if let Some(rest) = strip_command(message, command) {
            let mut route =
                finalize_route(intent, rest.to_string(), rest.to_string(), has_document, has_index);
            route.all_notes = command == "/search-all";
            return route;
        }
    }

    if is_small_talk(message) || (!has_document && !has_index) {
        return finalize_route(Intent::General, String::new(), message.to_string(), has_document, has_index);
    }

    let input = classifier_input(message, document_title, history);
    let classified: Result<IntentResponse, String> = match provider {
        ProviderKind::Local => match model.classify_intent(model_id, app_handle.clone()).await {
            Ok(session) => session(&input).await.map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        },
        ProviderKind::OpenRouter => match providers::load_api_key() {
            Ok(Some(api_key)) => {
                let model_name = openrouter_model
                    .clone()
                    .unwrap_or_else(|| providers::DEFAULT_OPENROUTER_MODEL.to_string());
                providers::chat_completion_json::<IntentResponse>(
                    &api_key,
                    &model_name,
                    INTENT_CLASSIFICATION_PROMPT,
                    &input,
                )
                .await
                .map_err(|e| e.to_string())
            }
            _ => Err("No OpenRouter API key set".to_string()),
        },
    };

    let (intent, query) = match classified {
        Ok(response) => (Intent::parse(&response.intent), response.query),
        Err(e) => {
            println!("WARN: message routing failed, answering as general chat: {e}");
            (Intent::General, String::new())
        }
    };

    finalize_route(intent, query, message.to_string(), has_document, has_index)
}

/// Corrects routes the context can't support (no document open, nothing indexed) and
/// makes sure a notes search always has something to embed.
fn finalize_route(
    intent: Intent,
    query: String,
    question: String,
    has_document: bool,
    has_index: bool,
) -> Route {
    let intent = match intent {
        Intent::CurrentDocument if !has_document => Intent::General,
        Intent::SearchNotes if !has_index && has_document => Intent::CurrentDocument,
        Intent::SearchNotes if !has_index => Intent::General,
        other => other,
    };

    let query = match intent {
        Intent::SearchNotes if query.trim().is_empty() => question.clone(),
        Intent::SearchNotes => query.trim().to_string(),
        _ => String::new(),
    };

    Route { intent, all_notes: false, query, question }
}

/// The binder (top-level folder) a note lives in, or `None` for a note at the top level.
fn binder_of(title: &str) -> Option<&str> {
    title.split_once('/').map(|(binder, _)| binder).filter(|b| !b.is_empty())
}

/// Returns the text after `command` when the message starts with it followed by
/// whitespace and something to act on, e.g. `/search dragon lore` -> `dragon lore`.
fn strip_command<'a>(message: &'a str, command: &str) -> Option<&'a str> {
    let rest = message.strip_prefix(command)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim();
    (!rest.is_empty()).then_some(rest)
}

fn is_small_talk(message: &str) -> bool {
    const PHRASES: &[&str] = &[
        "hi", "hello", "hey", "yo", "thanks", "thank you", "thx", "ok", "okay", "cool", "nice",
        "great", "good morning", "good evening", "good night", "bye",
    ];
    let normalized = message
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .trim()
        .to_lowercase();
    PHRASES.contains(&normalized.as_str())
}

fn role_label(role: &str) -> &'static str {
    if role == "assistant" {
        "Assistant"
    } else {
        "User"
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((idx, _)) => format!("{}…", &text[..idx]),
        None => text.to_string(),
    }
}

/// The exact input format the router sees; `llm::classify_intent`'s examples mirror it.
fn classifier_input(message: &str, document_title: Option<&str>, history: &[ChatTurn]) -> String {
    let document = document_title
        .map(|title| format!("\"{}\"", pretty_title(title)))
        .unwrap_or_else(|| "none".to_string());

    let start = history.len().saturating_sub(CLASSIFIER_HISTORY_TURNS);
    let conversation = if history.is_empty() {
        "none".to_string()
    } else {
        history[start..]
            .iter()
            .map(|turn| {
                format!(
                    "{}: {}",
                    role_label(&turn.role),
                    truncate_chars(&turn.content, CLASSIFIER_TURN_CHARS)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!("Open document: {document}\nRecent conversation:\n{conversation}\nLatest message: {message}")
}

/// Keeps only real user/assistant turns with content, the most recent few, each capped.
fn recent_history(history: &[ChatTurn]) -> Vec<ChatTurn> {
    let turns: Vec<&ChatTurn> = history
        .iter()
        .filter(|turn| (turn.role == "user" || turn.role == "assistant") && !turn.content.trim().is_empty())
        .collect();
    let start = turns.len().saturating_sub(HISTORY_TURNS);
    turns[start..]
        .iter()
        .map(|turn| ChatTurn {
            role: turn.role.clone(),
            content: truncate_chars(turn.content.trim(), HISTORY_TURN_CHARS),
        })
        .collect()
}

/// Local models get a single fresh chat per reply, so prior turns ride along as a
/// transcript. OpenRouter receives them as real messages instead.
fn with_transcript(history: &[ChatTurn], message: &str) -> String {
    if history.is_empty() {
        return message.to_string();
    }
    let transcript = history
        .iter()
        .map(|turn| format!("{}: {}", role_label(&turn.role), turn.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("<conversation_so_far>\n{transcript}\n</conversation_so_far>\n\n{message}")
}

fn build_rag_message(results: &[SearchResult], question: &str) -> String {
    let context = if results.is_empty() {
        "(No passages in the user's notes matched this question.)".to_string()
    } else {
        results
            .iter()
            .map(|result| {
                let mut attrs = format!("note=\"{}\"", pretty_title(&result.title));
                if !result.section.is_empty() {
                    attrs.push_str(&format!(" section=\"{}\"", result.section.replace('"', "'")));
                }
                if result.start_line > 0 {
                    attrs.push_str(&format!(" lines=\"{}-{}\"", result.start_line, result.end_line));
                }
                format!("<excerpt {attrs}>\n{}\n</excerpt>", result.content.trim())
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    // Question first: note text inserted afterwards can't be mistaken for a placeholder.
    RAG_CHAT_PROMPT
        .replace("{question}", question)
        .replace("{context}", &context)
}

fn build_document_message(title: &str, content: &str, question: &str) -> String {
    let document = if content.chars().count() > MAX_DOCUMENT_CHARS {
        format!(
            "{}\n\n[Document truncated: only the first part is shown.]",
            truncate_chars(content, MAX_DOCUMENT_CHARS)
        )
    } else {
        content.to_string()
    };
    DOCUMENT_CHAT_PROMPT
        .replace("{title}", &pretty_title(title))
        .replace("{question}", question)
        .replace("{document}", &document)
}

/// One entry per note (its best-scoring chunk), best first, capped for display.
fn source_refs(results: &[SearchResult]) -> Vec<serde_json::Value> {
    let mut seen = std::collections::HashSet::new();
    results
        .iter()
        .filter(|result| seen.insert(result.title.as_str()))
        .take(MAX_SOURCES)
        .map(|result| serde_json::json!({ "title": result.title, "score": result.score }))
        .collect()
}

/// Loads the OpenRouter API key from the OS keyring, mapping a missing key to a
/// user-facing error string.
fn openrouter_api_key() -> Result<String, String> {
    providers::load_api_key()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No OpenRouter API key set. Add one in Settings.".to_string())
}

/// Streams an OpenRouter chat completion, emitting `in_progress_event` for each chunk
/// as it arrives and `completed_event` once the stream ends. Mirrors the local Kalosm
/// streaming loop so the frontend sees the same event shape from either provider.
#[allow(clippy::too_many_arguments)]
async fn stream_openrouter(
    openrouter_model: &Option<String>,
    sys_prompt: &str,
    history: Vec<ChatTurn>,
    message: String,
    cancellation_token: &CancellationToken,
    app_handle: &AppHandle,
    in_progress_event: &'static str,
    completed_event: &'static str,
) -> Result<(), String> {
    let api_key = openrouter_api_key()?;
    let model_name = openrouter_model
        .clone()
        .unwrap_or_else(|| providers::DEFAULT_OPENROUTER_MODEL.to_string());

    let mut rx = providers::stream_chat_completion(
        api_key,
        model_name,
        sys_prompt.to_string(),
        history,
        message,
    )
    .await;

    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                return Err("Chat cancelled".to_string());
            }
            chunk = rx.recv() => {
                match chunk {
                    Some(Ok(token)) => {
                        app_handle
                            .emit(in_progress_event, ChatStreamInProgress { content: &token })
                            .unwrap();
                    }
                    Some(Err(e)) => {
                        return Err(e.to_string());
                    }
                    None => {
                        app_handle.emit(completed_event, "").unwrap();
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Represents the status of a job in the LLM event service.
/// It can be queued, running, completed with a result, failed with an error, or cancelled.
#[derive(serde::Serialize, Clone)]
pub enum JobStatus {
    Queued,
    Running,
    Completed(String),
    Failed(String),
    Cancelled,
}

/// Events emitted during the chat processing lifecycle.
/// Includes in-progress updates, completion, and error events.
/// Each event can be converted to a string representation for emission.
pub enum ChatEvents {
    Init,
    InProgress,
    Completed,
    AutoComplete,
    GrammarCheck,

    EditActionStart,
    EditActionInProgress,
    EditActionCompleted,

    /// How the router decided to answer (`intent`, `query`, `documentTitle`), emitted
    /// before any search runs so the UI can say what the assistant is doing.
    Route,
    /// Distinct source notes (`title`, `score`) for a reply that searched notes.
    Sources,

    Error,
}

impl ChatEvents {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatEvents::EditActionStart => "chat-edit-action-start",
            ChatEvents::EditActionInProgress => "chat-edit-action-in-progress",
            ChatEvents::EditActionCompleted => "chat-edit-action-completed",

            ChatEvents::Init => "chat-init",
            ChatEvents::AutoComplete => "chat-autocomplete",
            ChatEvents::GrammarCheck => "chat-grammar-check",
            ChatEvents::InProgress => "chat-in-progress",
            ChatEvents::Completed => "chat-completed",
            ChatEvents::Error => "chat-error",

            ChatEvents::Route => "chat-route",
            ChatEvents::Sources => "chat-sources",
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStreamInProgress<'a> {
    pub content: &'a str,
}

#[derive(Clone, Serialize)]
pub struct AutoCompleteStreamInProgress {
    response: AutoCompleteResponse,
}

#[derive(Clone, Serialize)]
pub struct GrammarCheckStreamInProgress<'a> {
    pub corrections: &'a str,
    pub explanation: &'a str,
}

/// Events emitted during the model loading lifecycle.
/// Includes downloading, loading, loaded, and error events.
pub enum ModelEvents {
    Downloading,
    Loading,
    Loaded,
    Error,
}

impl ModelEvents {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelEvents::Downloading => "model-downloading",
            ModelEvents::Loading => "model-loading",
            ModelEvents::Loaded => "model-loaded",
            ModelEvents::Error => "model-error",
        }
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    use surrealdb::sql::Thing;

    fn turn(role: &str, content: &str) -> ChatTurn {
        ChatTurn {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    fn result(title: &str, content: &str, score: f32) -> SearchResult {
        SearchResult {
            parent: Thing::from(("documents", title)),
            content: content.to_string(),
            sequence: 0,
            title: title.to_string(),
            score,
            section: String::new(),
            start_line: 0,
            end_line: 0,
        }
    }

    #[test]
    fn strip_command_needs_a_separator_and_an_argument() {
        assert_eq!(strip_command("/search dragon lore", "/search"), Some("dragon lore"));
        assert_eq!(strip_command("/search   spaced  ", "/search"), Some("spaced"));
        assert_eq!(strip_command("/searching things", "/search"), None);
        assert_eq!(strip_command("/search", "/search"), None);
        assert_eq!(strip_command("/search   ", "/search"), None);
        assert_eq!(strip_command("please /search x", "/search"), None);
    }

    #[test]
    fn binder_is_the_top_level_folder() {
        assert_eq!(binder_of("Novel/Part 1/Ch 3.md"), Some("Novel"));
        assert_eq!(binder_of("Novel/Ch 1.md"), Some("Novel"));
        assert_eq!(binder_of("Ideas.md"), None);
    }

    #[test]
    fn search_all_command_widens_the_search() {
        assert_eq!(strip_command("/search-all dragons", "/search"), None);
        assert_eq!(strip_command("/search-all dragons", "/search-all"), Some("dragons"));
    }

    #[test]
    fn small_talk_matches_whole_greetings_only() {
        assert!(is_small_talk("Hi!"));
        assert!(is_small_talk("  Thank you. "));
        assert!(!is_small_talk("hi, where did I write about the storm?"));
        assert!(!is_small_talk("summarize this"));
    }

    #[test]
    fn finalize_route_falls_back_when_context_is_missing() {
        let doc_without_document = finalize_route(Intent::CurrentDocument, String::new(), "summarize this".into(), false, true);
        assert_eq!(doc_without_document.intent, Intent::General);

        let search_without_index = finalize_route(Intent::SearchNotes, "storm".into(), "q".into(), true, false);
        assert_eq!(search_without_index.intent, Intent::CurrentDocument);
        assert_eq!(search_without_index.query, "");

        let search_without_anything = finalize_route(Intent::SearchNotes, "storm".into(), "q".into(), false, false);
        assert_eq!(search_without_anything.intent, Intent::General);
    }

    #[test]
    fn finalize_route_always_gives_a_search_something_to_embed() {
        let blank_query = finalize_route(Intent::SearchNotes, "  ".into(), "where is the lighthouse scene".into(), false, true);
        assert_eq!(blank_query.intent, Intent::SearchNotes);
        assert_eq!(blank_query.query, "where is the lighthouse scene");

        let general = finalize_route(Intent::General, "leftover".into(), "hi".into(), true, true);
        assert_eq!(general.query, "");
    }

    #[test]
    fn intent_parse_defaults_to_general() {
        assert_eq!(Intent::parse(" search_notes "), Intent::SearchNotes);
        assert_eq!(Intent::parse("current_document"), Intent::CurrentDocument);
        assert_eq!(Intent::parse("SEARCH"), Intent::General);
    }

    #[test]
    fn truncate_chars_is_utf8_safe() {
        assert_eq!(truncate_chars("héllo wörld", 4), "héll…");
        assert_eq!(truncate_chars("short", 10), "short");
    }

    #[test]
    fn recent_history_keeps_last_real_turns_and_caps_length() {
        let mut history: Vec<ChatTurn> = (0..10).map(|i| turn(if i % 2 == 0 { "user" } else { "assistant" }, &format!("m{i}"))).collect();
        history.push(turn("system", "ignore me"));
        history.push(turn("user", "   "));
        history.push(turn("assistant", &"x".repeat(HISTORY_TURN_CHARS + 50)));

        let recent = recent_history(&history);

        assert_eq!(recent.len(), HISTORY_TURNS);
        assert_eq!(recent[0].content, "m5");
        assert!(recent.iter().all(|t| t.role != "system"));
        assert_eq!(recent.last().unwrap().content.chars().count(), HISTORY_TURN_CHARS + 1);
    }

    #[test]
    fn classifier_input_describes_document_and_conversation() {
        let none = classifier_input("hi there", None, &[]);
        assert_eq!(none, "Open document: none\nRecent conversation:\nnone\nLatest message: hi there");

        let with_doc = classifier_input("and her sister?", Some("Novel/Mara.md"), &[turn("user", "who is Mara?"), turn("assistant", "The captain.")]);
        assert_eq!(
            with_doc,
            "Open document: \"Novel › Mara\"\nRecent conversation:\nUser: who is Mara?\nAssistant: The captain.\nLatest message: and her sister?"
        );
    }

    #[test]
    fn source_refs_lists_each_note_once_best_first() {
        let results = vec![
            result("A.md", "a1", 0.9),
            result("A.md", "a2", 0.85),
            result("B.md", "b1", 0.8),
            result("C.md", "c1", 0.79),
            result("D.md", "d1", 0.78),
        ];

        let titles: Vec<String> = source_refs(&results)
            .iter()
            .map(|r| r["title"].as_str().unwrap().to_string())
            .collect();

        assert_eq!(titles, vec!["A.md", "B.md", "C.md"]);
    }

    #[test]
    fn rag_message_names_notes_and_keeps_note_text_literal() {
        let message = build_rag_message(&[result("Lore/Dragons.md", "They hate {question} marks.", 0.9)], "why do dragons hoard?");

        assert!(message.contains("<excerpt note=\"Lore › Dragons\">\nThey hate {question} marks.\n</excerpt>"));
        assert!(message.contains("<question>\nwhy do dragons hoard?\n</question>"));
    }

    #[test]
    fn rag_message_cites_section_and_lines_when_known() {
        let mut hit = result("Novel/Ch 3.md", "She ran.", 0.9);
        hit.section = "Part 1 › Scene 2".to_string();
        hit.start_line = 12;
        hit.end_line = 18;
        let message = build_rag_message(&[hit], "what happens?");

        assert!(message.contains(
            "<excerpt note=\"Novel › Ch 3\" section=\"Part 1 › Scene 2\" lines=\"12-18\">\nShe ran.\n</excerpt>"
        ));
    }

    #[test]
    fn rag_message_says_when_nothing_matched() {
        let message = build_rag_message(&[], "anything about whales?");
        assert!(message.contains("No passages in the user's notes matched"));
    }

    #[test]
    fn document_message_truncates_long_documents() {
        let long = "a".repeat(MAX_DOCUMENT_CHARS + 10);
        let message = build_document_message("Draft.md", &long, "tighten this");

        assert!(message.contains("The user has the document \"Draft\" open"));
        assert!(message.contains("[Document truncated"));
        assert!(!message.contains(&"a".repeat(MAX_DOCUMENT_CHARS + 1)));
    }

    #[test]
    fn transcript_is_only_added_when_there_is_history() {
        assert_eq!(with_transcript(&[], "hello"), "hello");
        let with = with_transcript(&[turn("user", "hi"), turn("assistant", "hey")], "next");
        assert_eq!(with, "<conversation_so_far>\nUser: hi\n\nAssistant: hey\n</conversation_so_far>\n\nnext");
    }
}
