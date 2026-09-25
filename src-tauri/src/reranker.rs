//! Cross-encoder reranking for note search.
//!
//! Embedding search compares a query and a passage that were each encoded on
//! their own. A cross-encoder reads the query and the passage together, which
//! judges relevance much better but is too slow to run over every passage, so
//! it only rescores the few dozen candidates `Memory::search_hybrid` found.
//!
//! The model is `cross-encoder/ms-marco-MiniLM-L-6-v2` (22M parameters,
//! English). It runs on the same device as the other models: Metal on Apple
//! Silicon, the CPU elsewhere. On the CPU, rescoring a search's 16 candidates
//! takes up to about 1.8s in a release build when every passage is full-size.
//! It downloads (~90MB) into the same cache as the other models on first use.

use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::{Linear, Module, VarBuilder};
use candle_transformers::models::bert::{BertModel, Config};
use kalosm_common::Cache;
use kalosm_model_types::FileSource;
use std::path::{Path, PathBuf};
use tokenizers::{
    PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy,
};

const MODEL_ID: &str = "cross-encoder/ms-marco-MiniLM-L-6-v2";
const REVISION: &str = "main";

/// Tokens per query + passage pair. Passages are cut from the end to fit. A
/// full-size passage (`CHUNK_SIZE_LIMIT` bytes of prose) is about 230 tokens,
/// and cost grows faster than linearly with length.
const MAX_TOKENS: usize = 256;

/// Longest query passed in, so the query alone never fills `MAX_TOKENS`.
const MAX_QUERY_CHARS: usize = 400;

pub struct Reranker {
    model: BertModel,
    pooler: Linear,
    classifier: Linear,
    tokenizer: Tokenizer,
    device: Device,
}

impl Reranker {
    /// Downloads the model if needed and loads it.
    pub async fn load() -> Result<Self, String> {
        let cache = Cache::default();
        let mut paths: Vec<PathBuf> = Vec::new();
        for file in ["config.json", "tokenizer.json", "model.safetensors"] {
            let source = FileSource::huggingface(MODEL_ID, REVISION, file);
            let path = cache
                .get(&source, |_| {})
                .await
                .map_err(|e| format!("Could not download reranker {file}: {e}"))?;
            paths.push(path);
        }
        tokio::task::spawn_blocking(move || Self::from_files(&paths[0], &paths[1], &paths[2]))
            .await
            .map_err(|e| e.to_string())?
    }

    fn from_files(config: &Path, tokenizer: &Path, weights: &Path) -> Result<Self, String> {
        let config: Config = serde_json::from_str(
            &std::fs::read_to_string(config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("Bad reranker config: {e}"))?;

        let mut tokenizer = Tokenizer::from_file(tokenizer).map_err(|e| e.to_string())?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAX_TOKENS,
                strategy: TruncationStrategy::OnlySecond,
                ..Default::default()
            }))
            .map_err(|e| e.to_string())?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            ..Default::default()
        }));

        // The GPU on Apple Silicon (Metal), the CPU elsewhere; the same device the
        // embedder and local model use.
        let device = kalosm_common::accelerated_device_if_available().map_err(|e| e.to_string())?;
        // SAFETY: the weights file lives in the model cache and isn't modified
        // while the app runs.
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights], DType::F32, &device) }
            .map_err(|e| e.to_string())?;
        let hidden = config.hidden_size;
        let model = BertModel::load(vb.pp("bert"), &config).map_err(|e| e.to_string())?;
        // BertForSequenceClassification's head: pooler over [CLS], then one logit.
        let pooler = candle_nn::linear(hidden, hidden, vb.pp("bert.pooler.dense"))
            .map_err(|e| e.to_string())?;
        let classifier =
            candle_nn::linear(hidden, 1, vb.pp("classifier")).map_err(|e| e.to_string())?;

        Ok(Self { model, pooler, classifier, tokenizer, device })
    }

    /// How relevant each passage is to `query`, as raw logits: higher is more
    /// relevant, and above 0 is usually a real answer. Runs the model, so call
    /// it off the async runtime.
    pub fn score(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, String> {
        if passages.is_empty() {
            return Ok(Vec::new());
        }
        let query: String = query.chars().take(MAX_QUERY_CHARS).collect();
        let pairs: Vec<(String, String)> =
            passages.iter().map(|p| (query.clone(), p.clone())).collect();
        let encodings = self.tokenizer.encode_batch(pairs, true).map_err(|e| e.to_string())?;

        let rows = encodings.len();
        let cols = encodings[0].get_ids().len();
        let tensor = |pick: fn(&tokenizers::Encoding) -> &[u32]| {
            let flat: Vec<u32> = encodings.iter().flat_map(|e| pick(e).to_vec()).collect();
            Tensor::from_vec(flat, (rows, cols), &self.device)
        };
        let run = || -> candle_core::Result<Vec<f32>> {
            let ids = tensor(|e| e.get_ids())?;
            let type_ids = tensor(|e| e.get_type_ids())?;
            let mask = tensor(|e| e.get_attention_mask())?;
            let hidden = self.model.forward(&ids, &type_ids, Some(&mask))?;
            // Contiguous copy of the [CLS] row: Metal's matmul rejects the strided view.
            let cls = hidden.i((.., 0))?.contiguous()?;
            let pooled = self.pooler.forward(&cls)?.tanh()?;
            self.classifier.forward(&pooled)?.squeeze(1)?.to_vec1::<f32>()
        };
        run().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Downloads the model (~90MB) on first run: `cargo test -- --ignored reranker`.
    #[tokio::test]
    #[ignore]
    async fn reranker_prefers_the_passage_that_answers() {
        // Holds the GPU throughout, as app code does, so it can run beside the other tests.
        let _gpu = crate::gpu::lock().await;
        let reranker = Reranker::load().await.unwrap();
        let passages = vec![
            "Soup: simmer the carrots for twenty minutes.".to_string(),
            "The dragon guards its gold in a cave under the northern mountain.".to_string(),
            "Dragons appear in many myths.".to_string(),
        ];
        let started = std::time::Instant::now();
        let scores = reranker.score("where does the dragon keep its gold?", &passages).unwrap();
        println!("scores {scores:?} in {:?}", started.elapsed());
        assert_eq!(scores.len(), 3);
        assert!(scores[1] > scores[2] && scores[2] > scores[0]);

        // Timing for a full search: `RERANK_POOL` passages of the largest size the
        // chunker makes. Meaningful only in release: `cargo test --release`.
        let long = "She walked along the harbour wall, counting the boats. ".repeat(18);
        assert!(long.len() <= crate::chunker::CHUNK_SIZE_LIMIT);
        let batch: Vec<String> = (0..16).map(|i| format!("Novel › Part {i}\n{long}")).collect();
        let started = std::time::Instant::now();
        reranker.score("where did she walk?", &batch).unwrap();
        println!("16 full passages in {:?}", started.elapsed());
    }
}
