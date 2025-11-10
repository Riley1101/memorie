use crate::utils;

use super::error::LlamaError;
use kalosm::language::*;
use kalosm_common::Cache;
use std::path::PathBuf;

pub struct Model {
    name: PathBuf,
    llma: Option<Llama>,
    base_path: PathBuf,
}

impl Model {
    pub fn new(name: PathBuf) -> Self {
        let root_dir = utils::get_app_dir().expect("Failed to get app directory");

        Model {
            name,
            llma: None,
            base_path: root_dir,
        }
    }

    // !TODO use model from config
    pub async fn load_model(&self) -> Result<Llama, LlamaError> {
        let root_dir = self.base_path.clone().join("models/");

        let modal_path = PathBuf::from(root_dir);

        let cache = Cache::new(modal_path);

        let local_source = LlamaSource::qwen_2_5_0_5b_instruct().with_cache(cache);

        let loaded_model = Llama::builder()
            .with_source(local_source)
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

        Ok(loaded_model)
    }

    #[allow(dead_code)]
    pub fn is_model_loaded(&self) -> bool {
        self.llma.is_some()
    }

    pub fn set_loaded_model(&mut self, model: Llama) {
        self.llma = Some(model);
    }

    /// Runs a chat session with the provided model.
    /// And load previous session if exists.
    ///
    /// Returns a Chat instance.
    pub async fn run_chat(&self) -> Result<Chat<Llama>, LlamaError> {
        let session_cache_path = self.base_path.clone().join("chat_sessions/");

        let model = Llama::new_chat().await?;

        let mut chat = model
            .chat()
            .with_system_prompt("You are a helpful assistant.");

        if let Some(old_session) = std::fs::read(&session_cache_path)
            .ok()
            .and_then(|bytes| LlamaChatSession::from_bytes(&bytes).ok())
        {
            chat = chat.with_session(old_session);
        }

        Ok(chat)
    }
}
