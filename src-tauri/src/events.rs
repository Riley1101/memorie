use serde::Serialize;
use tauri::{AppHandle, Emitter};

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
