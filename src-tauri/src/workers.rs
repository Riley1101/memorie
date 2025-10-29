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
    pub message: String,
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

const MAX_CONCURRENT_LLM_CALLS: usize = 2;

impl LlmEventService {
    /// Creates a new LLM event service.
    /// Initializes the job queue, status map, and cancellation token map.
    /// Spawns a background task to process jobs from the queue with concurrency control.
    /// # Arguments
    /// * `app_handle` - A handle to the Tauri application for emitting events.
    /// # Returns
    /// A new instance of `LlmEventService`.
    pub fn new(app_handle: AppHandle) -> Self {
        let (sender, mut receiver) = mpsc::channel::<Job>(100); // Bounded channel for up to 100 queued jobs
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
                        res = run_chat_worker(app_handle.clone(), job.message.clone(), job.cancellation_token.clone()) => {
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
/// * `cancellation_token` - A token to signal cancellation of the chat processing.
/// # Returns
/// A `Result` containing the final response string or an error message.
async fn run_chat_worker(
    app_handle: AppHandle,
    message: String,
    cancellation_token: CancellationToken,
) -> Result<String, String> {
    let state: tauri::State<crate::AppState> = app_handle.state();
    let model = state.model.lock().await;

    let mut chat_session = model.run_chat().await.map_err(|e| e.to_string())?;
    let mut stream = chat_session.add_message(message);
    let mut response = String::new();

    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                println!("Chat cancelled");
                return Err("Chat was cancelled".to_string());
            }
            token = stream.next() => {
                match token {
                    Some(token) => {
                        response.push_str(&token);
                        app_handle
                            .emit(
                                ChatEvents::InProgress.as_str(),
                                ChatStreamInProgress { content: &token },
                            )
                            .unwrap();
                    }
                    None => {
                        break;
                    }
                }
            }
        }
    }
    app_handle.emit(ChatEvents::Completed.as_str(), "").unwrap();
    Ok(response)
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
    InProgress,
    Completed,
    Error,
}

impl ChatEvents {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatEvents::InProgress => "chat-in-progress",
            ChatEvents::Completed => "chat-completed",
            ChatEvents::Error => "chat-error",
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStreamInProgress<'a> {
    pub content: &'a str,
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
