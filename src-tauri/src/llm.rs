use kalosm::language::*;
use kalosm_common::Cache;
use std::path::PathBuf;
use futures::StreamExt;


const MODAL_CACHE_PATH: &str = "/home/arkar/.memorie/models/";

pub struct Model {
    name: PathBuf,
    llma: Option<Llama>,
}

impl Model {
    pub fn new(name: PathBuf) -> Self {
        Model { name,
            llma: None
        }
    }

    // !TODO use this.error  handling
    pub async fn load_model(&self) -> Result<Llama, Box<dyn std::error::Error>> {
        let modal_path = PathBuf::from(MODAL_CACHE_PATH);

        let cache = Cache::new(modal_path);

        let local_source = LlamaSource::qwen_2_5_0_5b_instruct().with_cache(cache);

        let loaded_model = Llama::builder().with_source(local_source).build().await?;

        Ok(loaded_model)
    }

    #[allow(dead_code)]
    pub fn is_model_loaded(&self) -> bool {
        self.llma.is_some()
    }

    pub fn set_loaded_model(&mut self, model: Llama) {
        self.llma = Some(model);
    }

    /// Runs a chat session with the provided model and user message.
    ///
    /// # Arguments
    /// * `user_message` - The message from the user to send to the model.
    ///
    /// # Returns
    /// A `Result` containing the model's string response or an error.
    pub async fn run_chat(&self, user_message: &str) -> Result<String, Box<dyn std::error::Error>> {
        let model = self
            .llma
            .as_ref()
            .ok_or("Model not loaded. Please load the model first.")?;

        let mut chat = model
            .chat()
            .with_system_prompt("You are an helpful assistant.");

        let mut response_stream = chat.add_message(user_message);

        let mut full_response = String::new();

        while let Some(token) = response_stream.next().await{
            println!("{:?}",token);
            full_response.push_str(&token);
        }
        Ok(full_response)
    }
}
