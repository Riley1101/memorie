use super::error::LlamaError;
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

    // !TODO use model from config
    /// Load or get the model from cache
    /// # Returns
    /// A Result containing a reference to the Llama model or a LlamaError
    pub async fn download_or_load_model(
        &mut self,
        model_type: ModelType,
    ) -> Result<Response<ModelLoadingResponse>, LlamaError> {
        if self.auto_complete_model.is_none() {
            let root_dir = self.base_path.clone().join("models/");
            let modal_path = PathBuf::from(root_dir);
            let cache = Cache::new(modal_path);

            let source = match &model_type {
                ModelType::Chat => LlamaSource::qwen_2_5_1_5b_instruct().with_cache(cache),
                ModelType::AutoComplete => LlamaSource::qwen_2_5_1_5b_instruct().with_cache(cache),
            };

            let loaded_model = Llama::builder()
                .with_source(source)
                .build_with_loading_handler(|progress| match progress {
                    ModelLoadingProgress::Downloading { source, progress } => {
                        let progress_percent = (progress.progress * 100) as u32;
                        let elapsed = progress.start_time.elapsed().as_secs_f32();
                        println!("Downloading file {source} {progress_percent}% ({elapsed}s)");
                    }
                    ModelLoadingProgress::Loading { progress } => {
                        let progress = (progress * 100.0) as u32;
                        println!("Loading model {progress}%");
                    }
                })
                .await?;
            match &model_type {
                ModelType::Chat => self.chat_model = Some(loaded_model),
                ModelType::AutoComplete => self.auto_complete_model = Some(loaded_model),
            }
        }

        let response = ModelLoadingResponse {
            is_loaded: true,
            message: match &model_type {
                ModelType::Chat => "Qwen 2.5B Instruct".to_string(),
                ModelType::AutoComplete => "Qwen 2.5B Instruct".to_string(),
            },
        };
        Ok(Response::success(response))
    }

    pub async fn get_model(&mut self, model_type: ModelType) -> Result<&Llama, LlamaError> {
        self.download_or_load_model(model_type.clone()).await?;
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
    ) -> Result<Task<Llama, ArcParser<AutoCompleteResponse>>, LlamaError> {
        let model = self.get_model(ModelType::AutoComplete).await?;

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
    ) -> Result<Chat<Llama>, LlamaError> {

        let prompt = if sys_prompt.is_empty() {
            NORMAL_CHAT_PROMPT.to_string()
        } else {
            sys_prompt.to_string()
        };

        let session_cache_path = self.base_path.clone().join("chat.llama");

        let model = self.get_model(ModelType::AutoComplete).await?;

        let mut chat = model
            .chat()
            .with_system_prompt(prompt);

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
    ) -> Result<Task<Llama, ArcParser<GrammarCheckResponse>>, LlamaError> {
        let model = self.get_model(ModelType::Chat).await?;

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
