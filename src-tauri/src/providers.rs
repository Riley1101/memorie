use super::error::ProviderError;
use super::utils::ChatTurn;
use futures_util::StreamExt;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Which backend serves chat/autocomplete/grammar requests. RAG embeddings and search
/// always stay local (Kalosm/rbert) regardless of this setting.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    #[default]
    Local,
    OpenRouter,
}

const KEYRING_SERVICE: &str = "memoire";
const KEYRING_USER: &str = "openrouter_api_key";
const OPENROUTER_API_BASE: &str = "https://openrouter.ai/api/v1";

/// Default model used until the user picks one in Settings.
pub const DEFAULT_OPENROUTER_MODEL: &str = "meta-llama/llama-3.1-8b-instruct:free";

fn key_entry() -> Result<Entry, ProviderError> {
    Ok(Entry::new(KEYRING_SERVICE, KEYRING_USER)?)
}

pub fn save_api_key(key: &str) -> Result<(), ProviderError> {
    key_entry()?.set_password(key)?;
    Ok(())
}

pub fn load_api_key() -> Result<Option<String>, ProviderError> {
    match key_entry()?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn clear_api_key() -> Result<(), ProviderError> {
    match key_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub name: String,
}

/// Fetches the list of models available on OpenRouter.
pub async fn fetch_models(api_key: &str) -> Result<Vec<OpenRouterModel>, ProviderError> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{OPENROUTER_API_BASE}/models"))
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!("{status}: {body}")));
    }

    let res: serde_json::Value = res.json().await?;

    let models = res["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?.to_string();
            let name = m["name"].as_str().unwrap_or(&id).to_string();
            Some(OpenRouterModel { id, name })
        })
        .collect();

    Ok(models)
}

/// Builds an OpenAI-style messages array: optional system prompt, prior turns (only
/// `user`/`assistant` roles are forwarded), then the new user message.
fn build_messages(
    system_prompt: &str,
    history: &[ChatTurn],
    message: &str,
) -> Vec<serde_json::Value> {
    let mut messages = Vec::with_capacity(history.len() + 2);
    if !system_prompt.is_empty() {
        messages.push(serde_json::json!({ "role": "system", "content": system_prompt }));
    }
    for turn in history {
        if turn.role == "user" || turn.role == "assistant" {
            messages.push(serde_json::json!({ "role": turn.role, "content": turn.content }));
        }
    }
    messages.push(serde_json::json!({ "role": "user", "content": message }));
    messages
}

/// Sends a single-turn chat completion request (no streaming). Returns the assistant's
/// full reply text.
pub async fn chat_completion(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    message: &str,
) -> Result<String, ProviderError> {
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{OPENROUTER_API_BASE}/chat/completions"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "messages": build_messages(system_prompt, &[], message),
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!("{status}: {body}")));
    }

    let body: serde_json::Value = res.json().await?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| ProviderError::Api("Response missing message content".to_string()))?
        .to_string();

    Ok(content)
}

/// Sends a streamed chat completion request. Chunks of the assistant's reply are sent
/// over the returned channel as they arrive (SSE, same wire format the OpenAI-compatible
/// APIs use); the channel closes when the stream ends or errors out.
pub async fn stream_chat_completion(
    api_key: String,
    model: String,
    system_prompt: String,
    history: Vec<ChatTurn>,
    message: String,
) -> mpsc::Receiver<Result<String, ProviderError>> {
    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        let messages = build_messages(&system_prompt, &history, &message);
        if let Err(e) = run_stream(&api_key, &model, messages, &tx).await {
            let _ = tx.send(Err(e)).await;
        }
    });

    rx
}

async fn run_stream(
    api_key: &str,
    model: &str,
    messages: Vec<serde_json::Value>,
    tx: &mpsc::Sender<Result<String, ProviderError>>,
) -> Result<(), ProviderError> {
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{OPENROUTER_API_BASE}/chat/completions"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!("{status}: {body}")));
    }

    let mut byte_stream = res.bytes_stream();
    // Raw bytes, not a String: a multi-byte UTF-8 character (emoji, accented letters, CJK)
    // can be split across two TCP chunks, and decoding each chunk independently would
    // mangle it. Buffering bytes and only decoding once a full line is assembled avoids that.
    let mut buf: Vec<u8> = Vec::new();

    while let Some(chunk) = byte_stream.next().await {
        let chunk = chunk?;
        buf.extend_from_slice(&chunk);

        // SSE frames are newline-delimited "data: {...}" lines.
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim();

            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                return Ok(());
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                    if tx.send(Ok(content.to_string())).await.is_err() {
                        // Receiver dropped (cancelled) — stop reading the stream.
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

/// Strips a ```json ... ``` (or plain ``` ... ```) fence if the model wrapped its
/// JSON reply in one, since chat models often do this despite being asked not to.
fn strip_code_fence(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        if let Some(end) = rest.rfind("```") {
            return rest[..end].trim();
        }
    }
    trimmed
}

/// Like `chat_completion`, but asks the model to reply with a JSON object matching `T`
/// and parses it. Used for grammar-check / autocomplete / routing, where local models use
/// Kalosm's typed structured generation instead.
pub async fn chat_completion_json<T: for<'de> Deserialize<'de>>(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    message: &str,
) -> Result<T, ProviderError> {
    let raw = chat_completion(api_key, model, system_prompt, message).await?;
    let candidate = strip_code_fence(&raw);

    if let Ok(v) = serde_json::from_str(candidate) {
        return Ok(v);
    }

    // Some models add a preamble ("Sure, here's the JSON:") despite being told not to.
    // Fall back to the outermost {...} block before giving up.
    if let (Some(start), Some(end)) = (candidate.find('{'), candidate.rfind('}')) {
        if start < end {
            if let Ok(v) = serde_json::from_str(&candidate[start..=end]) {
                return Ok(v);
            }
        }
    }

    Err(ProviderError::Api(format!(
        "Failed to parse JSON response: {candidate}"
    )))
}
