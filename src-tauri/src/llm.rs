use super::error::{FileError, LlamaError};
use serde::{Deserialize, Serialize};
use super::prompts::{GRAMMAR_CHECK_PROMPT, NORMAL_CHAT_PROMPT, TEXT_COMPLETION};
use super::responses::{AutoCompleteResponse, ModelLoadingResponse, Response};
use crate::responses::GrammarCheckResponse;
use crate::utils;
use kalosm::language::*;
use kalosm_common::Cache;
use std::path::PathBuf;

#[derive(Clone)]
pub enum ModelType {
    Chat,
    AutoComplete,
}

/// Model capability type for the UI (chat, reasoning, or coding).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModelTypeKind {
    Chat,
    Reasoning,
    Coding,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelStatus {
    pub name: String,
    pub downloaded: bool,
}

/// Supported model with type and download status for the settings UI.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupportedModel {
    pub id: String,
    pub name: String,
    pub model_type: ModelTypeKind,
    pub downloaded: bool,
}

/// Metadata for a Kalosm preset: (model_id, revision, file) for cache path.
struct SupportedModelInfo {
    id: &'static str,
    name: &'static str,
    model_type: ModelTypeKind,
    model_id: &'static str,
    revision: &'static str,
    file: &'static str,
}

/// Curated list of Kalosm-supported models with types (chat, reasoning, coding).
const SUPPORTED_MODELS: &[SupportedModelInfo] = &[
    // Chat models
    SupportedModelInfo {
        id: "qwen_2_5_0_5b_instruct",
        name: "Qwen 2.5 0.5B Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
        revision: "main",
        file: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
    },
    SupportedModelInfo {
        id: "qwen_2_5_1_5b_instruct",
        name: "Qwen 2.5 1.5B Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
        revision: "main",
        file: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
    },
    SupportedModelInfo {
        id: "qwen_2_5_3b_instruct",
        name: "Qwen 2.5 3B Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "Qwen/Qwen2.5-3B-Instruct-GGUF",
        revision: "main",
        file: "qwen2.5-3b-instruct-q4_k_m.gguf",
    },
    SupportedModelInfo {
        id: "qwen_2_5_7b_instruct",
        name: "Qwen 2.5 7B Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "bartowski/Qwen2.5-7B-Instruct-GGUF",
        revision: "main",
        file: "Qwen2.5-7B-Instruct-Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "phi_3_5_mini_4k_instruct",
        name: "Phi 3.5 Mini 4K Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "bartowski/Phi-3.5-mini-instruct-GGUF",
        revision: "main",
        file: "Phi-3.5-mini-instruct-Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "llama_3_2_1b_chat",
        name: "Llama 3.2 1B Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "lmstudio-community/Llama-3.2-1B-Instruct-GGUF",
        revision: "main",
        file: "Llama-3.2-1B-Instruct-Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "llama_3_2_3b_chat",
        name: "Llama 3.2 3B Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "lmstudio-community/Llama-3.2-3B-Instruct-GGUF",
        revision: "main",
        file: "Llama-3.2-3B-Instruct-Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "llama_3_1_8b_chat",
        name: "Llama 3.1 8B Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "lmstudio-community/Meta-Llama-3.1-8B-Instruct-GGUF",
        revision: "main",
        file: "Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "tiny_llama_1_1b_chat",
        name: "TinyLlama 1.1B Chat",
        model_type: ModelTypeKind::Chat,
        model_id: "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF",
        revision: "main",
        file: "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "mistral_7b_instruct",
        name: "Mistral 7B Instruct",
        model_type: ModelTypeKind::Chat,
        model_id: "TheBloke/Mistral-7B-Instruct-v0.1-GGUF",
        revision: "main",
        file: "mistral-7b-instruct-v0.1.Q4_K_M.gguf",
    },
    // Reasoning models (DeepSeek R1 distill)
    SupportedModelInfo {
        id: "deepseek_r1_distill_qwen_1_5b",
        name: "DeepSeek R1 Distill Qwen 1.5B",
        model_type: ModelTypeKind::Reasoning,
        model_id: "bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF",
        revision: "main",
        file: "DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "deepseek_r1_distill_qwen_7b",
        name: "DeepSeek R1 Distill Qwen 7B",
        model_type: ModelTypeKind::Reasoning,
        model_id: "bartowski/DeepSeek-R1-Distill-Qwen-7B-GGUF",
        revision: "main",
        file: "DeepSeek-R1-Distill-Qwen-7B-Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "deepseek_r1_distill_qwen_14b",
        name: "DeepSeek R1 Distill Qwen 14B",
        model_type: ModelTypeKind::Reasoning,
        model_id: "bartowski/DeepSeek-R1-Distill-Qwen-14B-GGUF",
        revision: "main",
        file: "DeepSeek-R1-Distill-Qwen-14B-Q4_K_M.gguf",
    },
    SupportedModelInfo {
        id: "deepseek_r1_distill_llama_8b",
        name: "DeepSeek R1 Distill Llama 8B",
        model_type: ModelTypeKind::Reasoning,
        model_id: "bartowski/DeepSeek-R1-Distill-Llama-8B-GGUF",
        revision: "main",
        file: "DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf",
    },
    // Coding models
    SupportedModelInfo {
        id: "llama_7b_code",
        name: "Code Llama 7B",
        model_type: ModelTypeKind::Coding,
        model_id: "TheBloke/CodeLlama-7B-GGUF",
        revision: "main",
        file: "codellama-7b.Q8_0.gguf",
    },
    SupportedModelInfo {
        id: "llama_13b_code",
        name: "Code Llama 13B",
        model_type: ModelTypeKind::Coding,
        model_id: "TheBloke/CodeLlama-13B-GGUF",
        revision: "main",
        file: "codellama-13b.Q8_0.gguf",
    },
    SupportedModelInfo {
        id: "llama_34b_code",
        name: "Code Llama 34B",
        model_type: ModelTypeKind::Coding,
        model_id: "TheBloke/CodeLlama-34B-GGUF",
        revision: "main",
        file: "codellama-34b.Q8_0.gguf",
    },
];

/// Returns a LlamaSource for the given model id (for downloading). Cache must be set via .with_cache().
fn source_for_model_id(model_id: &str) -> Result<LlamaSource, LlamaError> {
    let source = match model_id {
        "qwen_2_5_0_5b_instruct" => LlamaSource::qwen_2_5_0_5b_instruct(),
        "qwen_2_5_1_5b_instruct" => LlamaSource::qwen_2_5_1_5b_instruct(),
        "qwen_2_5_3b_instruct" => LlamaSource::qwen_2_5_3b_instruct(),
        "qwen_2_5_7b_instruct" => LlamaSource::qwen_2_5_7b_instruct(),
        "phi_3_5_mini_4k_instruct" => LlamaSource::phi_3_5_mini_4k_instruct(),
        "llama_3_2_1b_chat" => LlamaSource::llama_3_2_1b_chat(),
        "llama_3_2_3b_chat" => LlamaSource::llama_3_2_3b_chat(),
        "llama_3_1_8b_chat" => LlamaSource::llama_3_1_8b_chat(),
        "tiny_llama_1_1b_chat" => LlamaSource::tiny_llama_1_1b_chat(),
        "mistral_7b_instruct" => LlamaSource::mistral_7b_instruct(),
        "deepseek_r1_distill_qwen_1_5b" => LlamaSource::deepseek_r1_distill_qwen_1_5b(),
        "deepseek_r1_distill_qwen_7b" => LlamaSource::deepseek_r1_distill_qwen_7b(),
        "deepseek_r1_distill_qwen_14b" => LlamaSource::deepseek_r1_distill_qwen_14b(),
        "deepseek_r1_distill_llama_8b" => LlamaSource::deepseek_r1_distill_llama_8b(),
        "llama_7b_code" => LlamaSource::llama_7b_code(),
        "llama_13b_code" => LlamaSource::llama_13b_code(),
        "llama_34b_code" => LlamaSource::llama_34b_code(),
        _ => return Err(LlamaError::UnknownModel(model_id.to_string())),
    };
    Ok(source)
}

/// Downloads a model by id into the given cache dir. Emits model-progress events.
/// Does not require holding the Model lock, so check_models / get_supported_models can run during download.
pub async fn download_model_to_cache(
    model_id: &str,
    cache_dir: PathBuf,
    handle: tauri::AppHandle,
) -> Result<(), LlamaError> {
    let info = SUPPORTED_MODELS
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| LlamaError::UnknownModel(model_id.to_string()))?;

    std::fs::create_dir_all(&cache_dir).map_err(FileError::from)?;
    let cache = Cache::new(cache_dir);

    let source = FileSource::huggingface(info.model_id, info.revision, info.file);

    let handle_clone = handle.clone();
    let last_emitted_percent = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(101));

    let _path = cache
        .get(
            &source,
            move |progress: FileLoadingProgress| {
                use tauri::Emitter;
                let progress_percent = if progress.size > 0 {
                    (progress.progress as f64 / progress.size as f64 * 100.0) as u32
                } else {
                    0
                };
                let last = last_emitted_percent.load(std::sync::atomic::Ordering::Relaxed);
                if progress_percent != last {
                    last_emitted_percent
                        .store(progress_percent, std::sync::atomic::Ordering::Relaxed);
                    let _ = handle_clone.emit(
                        "model-progress",
                        serde_json::json!({
                            "status": "Downloading model...",
                            "progress": progress_percent
                        }),
                    );
                }
            },
        )
        .await?;
    Ok(())
}

pub struct Model {
    name: PathBuf,
    chat_model: Option<Llama>,
    auto_complete_model: Option<Llama>,
    base_path: PathBuf,
}

impl Model {
    pub fn new(name: PathBuf) -> Self {
        let root_dir = utils::get_app_dir().expect("Failed to get app directory");

        Model {
            name,
            chat_model: None,
            auto_complete_model: None,
            base_path: root_dir,
        }
    }

    /// Load or get the model from cache by preset id (e.g. "qwen_2_5_1_5b_instruct").
    pub async fn download_or_load_model_by_id(
        &mut self,
        model_id: &str,
        model_type: ModelType,
        handle: tauri::AppHandle,
    ) -> Result<Response<ModelLoadingResponse>, LlamaError> {
        let is_already_loaded = match &model_type {
            ModelType::Chat => self.chat_model.is_some(),
            ModelType::AutoComplete => self.auto_complete_model.is_some(),
        };

        if !is_already_loaded {
            let model_path = &self.name;
            let cache_dir = model_path.parent().unwrap_or(&self.base_path).to_path_buf();
            std::fs::create_dir_all(&cache_dir).map_err(FileError::from)?;
            let cache = Cache::new(cache_dir);

            let source = source_for_model_id(model_id)?.with_cache(cache);

            let handle_clone = handle.clone();
            let last_emitted_percent = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(101)); 

            let loaded_model = Llama::builder()
                .with_source(source)
                .build_with_loading_handler(move |progress| {
                    use tauri::Emitter;
                    match progress {
                        ModelLoadingProgress::Downloading { source, progress } => {
                            let progress_percent = if progress.size > 0 {
                                (progress.progress as f64 / progress.size as f64 * 100.0) as u32
                            } else {
                                0
                            };

                            let last = last_emitted_percent.load(std::sync::atomic::Ordering::Relaxed);
                            if progress_percent != last {
                                last_emitted_percent.store(progress_percent, std::sync::atomic::Ordering::Relaxed);
                                
                                let elapsed = progress.start_time.elapsed().as_secs_f32();
                                println!("Downloading file {source} {progress_percent}% ({elapsed}s)");
                                let _ = handle_clone.emit(
                                    "model-progress",
                                    serde_json::json!({
                                        "status": format!("Downloading model..."),
                                        "progress": progress_percent
                                    }),
                                );
                            }
                        }
                        ModelLoadingProgress::Loading { progress } => {
                            let progress = (progress * 100.0) as u32;
                            
                            let last = last_emitted_percent.load(std::sync::atomic::Ordering::Relaxed);
                            if progress != last {
                                last_emitted_percent.store(progress, std::sync::atomic::Ordering::Relaxed);

                                println!("Loading model {progress}%");
                                let _ = handle_clone.emit(
                                    "model-progress",
                                    serde_json::json!({
                                        "status": "Initializing model...",
                                        "progress": progress
                                    }),
                                );
                            }
                        }
                    }
                })
                .await?;
            match &model_type {
                ModelType::Chat => self.chat_model = Some(loaded_model),
                ModelType::AutoComplete => self.auto_complete_model = Some(loaded_model),
            }
        }

        let display_name = SUPPORTED_MODELS
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.name.to_string())
            .unwrap_or_else(|| model_id.to_string());
        let response = ModelLoadingResponse {
            is_loaded: true,
            message: display_name,
        };
        Ok(Response::success(response))
    }

    /// Load or get the model from cache (uses default preset).
    pub async fn download_or_load_model(
        &mut self,
        model_type: ModelType,
        handle: tauri::AppHandle,
    ) -> Result<Response<ModelLoadingResponse>, LlamaError> {
        self.download_or_load_model_by_id("qwen_2_5_1_5b_instruct", model_type, handle)
            .await
    }

    /// Returns status of the legacy single-model list (for backward compatibility).
    pub async fn check_models(&self) -> Vec<ModelStatus> {
        let supported = self.get_supported_models().await;
        supported
            .into_iter()
            .map(|m| ModelStatus {
                name: m.name,
                downloaded: m.downloaded,
            })
            .collect()
    }

    /// Returns all supported Kalosm models with their type and download status.
    pub async fn get_supported_models(&self) -> Vec<SupportedModel> {
        let root_dir = utils::get_app_dir().expect("Failed to get app directory");
        let model_path = &self.name;
        let cache_dir = model_path.parent().unwrap_or(&root_dir);

        SUPPORTED_MODELS
            .iter()
            .map(|info| {
                let full_path = cache_dir
                    .join(info.model_id)
                    .join(info.revision)
                    .join(info.file);
                SupportedModel {
                    id: info.id.to_string(),
                    name: info.name.to_string(),
                    model_type: info.model_type.clone(),
                    downloaded: full_path.exists(),
                }
            })
            .collect()
    }

    /// Downloads a model by id. Emits model-progress events via the given handle.
    /// Uses the same cache dir as this model (no lock held during download).
    pub async fn download_model(
        &self,
        model_id: &str,
        handle: tauri::AppHandle,
    ) -> Result<(), LlamaError> {
        let root_dir = utils::get_app_dir().expect("Failed to get app directory");
        let cache_dir = self.name.parent().unwrap_or(&root_dir).to_path_buf();
        download_model_to_cache(model_id, cache_dir, handle).await
    }

    pub async fn get_model(
        &mut self,
        model_type: ModelType,
        handle: tauri::AppHandle,
    ) -> Result<&Llama, LlamaError> {
        self.download_or_load_model(model_type.clone(), handle).await?;
        match &model_type {
            ModelType::Chat => Ok(self.chat_model.as_ref().unwrap()),
            ModelType::AutoComplete => Ok(self.auto_complete_model.as_ref().unwrap()),
        }
    }

    /// Load or create a chat session with the model
    /// First tries to load a previous session from cache
    /// If no previous session is found, creates a new chat session with a default system prompt
    ///
    /// # Returns
    /// A Result containing the Chat instance or a LlamaError
    pub async fn run_autocomplete(
        &mut self,
        handle: tauri::AppHandle,
    ) -> Result<Task<Llama, ArcParser<AutoCompleteResponse>>, LlamaError> {
        let model = self.get_model(ModelType::AutoComplete, handle).await?;

        let task = model
            .task(TEXT_COMPLETION.to_string())
            .with_example(
                "CONTEXT: Subject: Meeting Request. Hi Dave, I reviewed the quarterly reports and noticed some discrepancies in the marketing budget. CURRENT_INPUT: I would like to schedule a time to",
                "{ 'completion': 'discuss these figures before the board meeting next week.' }"
            )
            .with_example(
                "CONTEXT: To get started with the API, you first need to generate an authentication token in your dashboard. Once you have the key, include it in the header. CURRENT_INPUT: If the request is successful, the server will return",
                "{ 'completion': 'a 200 OK status code along with the requested JSON data.' }"
            )
            .with_example(
                "CONTEXT: The old house stood at the end of the lane, its windows boarded up and the garden overgrown with weeds. Nobody had lived there for fifty years. CURRENT_INPUT: As the storm approached, the front door suddenly",
                "{ 'completion': 'creaked open, revealing a flickering light inside.' }"
            )
            .with_example(
                "CONTEXT: While remote work offers flexibility, it also presents challenges regarding team cohesion. Spontaneous interactions are harder to replicate digitally. CURRENT_INPUT: Therefore, organizations must intentionally design",
                "{ 'completion': 'virtual spaces that foster casual communication and relationship building.' }"
            )
            .with_example(
                "CONTEXT: // This function calculates the fibonacci sequence recursively. // Note: This implementation is not optimized for large numbers. CURRENT_INPUT: // To improve performance, we should consider using",
                "{ 'completion': 'memoization or an iterative approach.' }"
            )
            .typed::<AutoCompleteResponse>();
        Ok(task)
    }

    /// Load or create a chat session with the model
    /// First tries to load a previous session from cache
    /// If no previous session is found, creates a new chat session with a default system prompt
    ///
    /// # Returns
    /// A Result containing the Chat instance or a LlamaError
    pub async fn run_chat(
        &mut self,
        sys_prompt: &str,
        handle: tauri::AppHandle,
    ) -> Result<Chat<Llama>, LlamaError> {
        let prompt = if sys_prompt.is_empty() {
            NORMAL_CHAT_PROMPT.to_string()
        } else {
            sys_prompt.to_string()
        };

        let session_cache_path = self.base_path.clone().join("chat.llama");

        let model = self.get_model(ModelType::Chat, handle).await?;
        let mut chat = model.chat().with_system_prompt(prompt);

        if let Some(old_session) = std::fs::read(&session_cache_path)
            .ok()
            .and_then(|bytes| LlamaChatSession::from_bytes(&bytes).ok())
        {
            chat = chat.with_session(old_session);
        }

        Ok(chat)
    }

    /// Load or create a grammar check session with the models
    /// First tries to load a previous session from Cache
    /// If no previous session is found, creates a new chat session with a grammar check system
    /// prompts
    /// # Returns
    /// A Result containing the Chat instance or a LlamaError
    pub async fn run_grammar_check(
        &mut self,
        handle: tauri::AppHandle,
    ) -> Result<Task<Llama, ArcParser<GrammarCheckResponse>>, LlamaError> {
        let model = self.get_model(ModelType::Chat, handle).await?;

        let task = model
            .task(GRAMMAR_CHECK_PROMPT.to_string())
            .with_example(
                "CONTEXT: Subject: Meeting Request. Hi Dave, I reviewed the quarterly reports and noticed some discrepancies in the marketing budget. CURRENT_INPUT: I would like to schedule a time to",
                "{ 'completion': 'discuss these figures before the board meeting next week.' }"
            )
            .with_example(
                "CONTEXT: To get started with the API, you first need to generate an authentication token in your dashboard. Once you have the key, include it in the header. CURRENT_INPUT: If the request is successful, the server will return",
                "{ 'completion': 'a 200 OK status code along with the requested JSON data.' }"
            )
            .with_example(
                "CONTEXT: The old house stood at the end of the lane, its windows boarded up and the garden overgrown with weeds. Nobody had lived there for fifty years. CURRENT_INPUT: As the storm approached, the front door suddenly",
                "{ 'completion': 'creaked open, revealing a flickering light inside.' }"
            )
            .with_example(
                "CONTEXT: While remote work offers flexibility, it also presents challenges regarding team cohesion. Spontaneous interactions are harder to replicate digitally. CURRENT_INPUT: Therefore, organizations must intentionally design",
                "{ 'completion': 'virtual spaces that foster casual communication and relationship building.' }"
            )
            .with_example(
                "CONTEXT: // This function calculates the fibonacci sequence recursively. // Note: This implementation is not optimized for large numbers. CURRENT_INPUT: // To improve performance, we should consider using",
                "{ 'completion': 'memoization or an iterative approach.' }"
            )
            .typed::<GrammarCheckResponse>();
        Ok(task)
    }
}
