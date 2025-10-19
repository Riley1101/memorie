use super::error::LlamaError;
use kalosm::language::*;
use kalosm_common::Cache;
use std::path::PathBuf;

const MODAL_CACHE_PATH: &str = "/home/arkar/.memorie/models/";
const CHAT_SESSION_CACHE: &str = "/home/arkar/.memorie/chat_sessions/";

pub struct Model {
    name: PathBuf,
    llma: Option<Llama>,
}

impl Model {
    pub fn new(name: PathBuf) -> Self {
        Model { name, llma: None }
    }

    // !TODO use this.error  handling
    pub async fn load_model(&self) -> Result<Llama, LlamaError> {
        let modal_path = PathBuf::from(MODAL_CACHE_PATH);

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
        let session_cache_path = PathBuf::from(CHAT_SESSION_CACHE);

        let model = Llama::new_chat().await?;

        let mut chat = model.chat();

        if let Some(old_session) = std::fs::read(&session_cache_path)
            .ok()
            .and_then(|bytes| LlamaChatSession::from_bytes(&bytes).ok())
        {
            chat = chat.with_session(old_session);
        }

        Ok(chat)
    }
}
