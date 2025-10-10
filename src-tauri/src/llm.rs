use kalosm::language::*;
use kalosm_common::Cache;
use std::path::PathBuf;

const MODAL_CACHE_PATH: &str = "/home/arkar/.memorie/models/";

pub struct Model {
    name: PathBuf,
}

// pub fn phi_3_mini_4k_instruct() -> Self {
//     Self::new(FileSource::huggingface(
//         "microsoft/Phi-3-mini-4k-instruct-gguf".to_string(),
//         "5eef2ce24766d31909c0b269fe90c817a8f263fb".to_string(),
//         "Phi-3-mini-4k-instruct-q4.gguf".to_string(),
//     ))
//     .with_tokenizer(FileSource::huggingface(
//         "microsoft/Phi-3-mini-4k-instruct".to_string(),
//         "main".to_string(),
//         "tokenizer.json".to_string(),
//     ))
//     .with_group_query_attention(1)
//     .with_override_stop_token_string("<|end|>".to_string())
// }

impl Model {
    pub fn new(name: PathBuf) -> Self {
        Model { name }
    }

    pub async fn load(&self) -> Result<String, Box<dyn std::error::Error>> {
        let modal_path = PathBuf::from(MODAL_CACHE_PATH);

        let cache = Cache::new(modal_path);

        let local_source = LlamaSource::tiny_llama_1_1b_chat().with_cache(cache);

        let model = Llama::builder().with_source(local_source).build().await?;

        let mut chat = model
            .chat()
            .with_system_prompt("Write a poem about the moon");

        let response = chat.add_message("Tell me 3 words about the moon").await?;

        Ok(response)
    }
}
