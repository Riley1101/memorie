use super::error::FileError;
use super::prompts::{
    CORRECT_GRAMMAR_PROMPT, IMPROVE_CLARITY_PROMPT, LENGTH_EXPAND_PROMPT, LENGTH_SHORTEN_PROMPT,
    LOGICAL_FLOW_PROMPT, MAKE_FORMAL_PROMPT, PROMPT_EXPANSION_PROMPT, SIMPLIFY_PROMPT,
    STYLE_CREATIVE_PROMPT, TONE_ACADEMIC_PROMPT, TONE_FRIENDLY_PROMPT,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Get the directory where the app stores its data.
pub fn get_app_dir() -> Result<PathBuf, FileError> {
    let home_dir = dirs::home_dir().ok_or(FileError::HomeDirNotFound)?;
    let app_dir = home_dir.join(".memorie/");
    Ok(app_dir)
}

/// One earlier message in the chat. The frontend sends the last few with every request,
/// so each reply is built from a short, clean history rather than a persisted model
/// session that would also accumulate every injected note excerpt and document.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

/// Everything the frontend knows about the conversation besides the message itself.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatContext {
    #[serde(default)]
    pub history: Vec<ChatTurn>,
    pub document_title: Option<String>,
    pub document_content: Option<String>,
}

/// Modes for chat interactions.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum ChatMode {
    Normal,
    Autocomplete,
    Grammar,
    EditAction,
}

/// Convert ChatMode to its string representation.
impl ChatMode {
    pub fn as_str(&self) -> &str {
        match self {
            ChatMode::Normal => "Normal",
            ChatMode::Autocomplete => "Autocomplete",
            ChatMode::Grammar => "Grammar",
            ChatMode::EditAction => "EditAction",
        }
    }
}

/// Editing actions that can be performed on text.
#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum EditAction {
    CorrectGrammar,
    ImproveClarity,
    MakeFormal,
    Simplify,
    PromptExpansion,
    ToneFriendly,
    ToneAcademic,
    LengthExpand,
    LengthShorten,
    StyleCreative,
    LogicalFlow,
}

impl EditAction {
    pub fn as_str(&self) -> &str {
        match self {
            EditAction::PromptExpansion => "PromptExpansion",
            EditAction::CorrectGrammar => "CorrectGrammar",
            EditAction::ImproveClarity => "ImproveClarity",
            EditAction::MakeFormal => "MakeFormal",
            EditAction::Simplify => "Simplify",
            EditAction::ToneFriendly => "ToneFriendly",
            EditAction::ToneAcademic => "ToneAcademic",
            EditAction::LengthExpand => "LengthExpand",
            EditAction::LengthShorten => "LengthShorten",
            EditAction::StyleCreative => "StyleCreative",
            EditAction::LogicalFlow => "LogicalFlow",
        }
    }

    pub fn into_prompt(&self) -> &str {
        match self {
            EditAction::PromptExpansion => PROMPT_EXPANSION_PROMPT,
            EditAction::CorrectGrammar => CORRECT_GRAMMAR_PROMPT,
            EditAction::ImproveClarity => IMPROVE_CLARITY_PROMPT,
            EditAction::MakeFormal => MAKE_FORMAL_PROMPT,
            EditAction::Simplify => SIMPLIFY_PROMPT,
            EditAction::ToneFriendly => TONE_FRIENDLY_PROMPT,
            EditAction::ToneAcademic => TONE_ACADEMIC_PROMPT,
            EditAction::LengthExpand => LENGTH_EXPAND_PROMPT,
            EditAction::LengthShorten => LENGTH_SHORTEN_PROMPT,
            EditAction::StyleCreative => STYLE_CREATIVE_PROMPT,
            EditAction::LogicalFlow => LOGICAL_FLOW_PROMPT,
        }
    }
}
