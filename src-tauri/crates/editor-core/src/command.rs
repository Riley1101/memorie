//! Commands: the whole vocabulary the host may use to change a document.
//!
//! The frontend never mutates the document; it sends one of these. That keeps
//! the set of ways a document can change small enough to test, and means a
//! second consumer of this engine gets the same editing behaviour for free.

use crate::inline::{ImageNode, MarkSet};
use crate::selection::SelectionRange;
use serde::{Deserialize, Serialize};

/// Which way a delete with a collapsed caret goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Backward,
    Forward,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "camelCase")]
pub enum EditorCommand {
    /// Moves the caret or selection. Recorded in history like any other edit,
    /// so undo puts the caret back where it was.
    SetSelection(SelectionRange),

    /// Types text at the caret, replacing the selection. Inherits the marks in
    /// force at the insertion point.
    InsertText(String),
    /// Deletes the selection, or one character in `direction` when collapsed.
    /// At a block edge a backward delete merges with the block before it.
    Delete(Direction),

    /// Splits the block at the caret. A heading's remainder becomes a
    /// paragraph, as in every editor that does this.
    SplitBlock,
    /// Joins the block at the caret onto the text block before it.
    MergeBlocks,

    /// Toggles a mark over the selection: on unless the whole selection
    /// already carries it.
    ToggleMark(MarkSet),

    /// `0` turns a heading back into a paragraph; `1..=6` set the level.
    SetHeadingLevel(u8),
    /// Turns the block at the caret into a code block, keeping its text.
    SetCodeBlock {
        #[serde(default)]
        language: Option<String>,
    },
    /// Turns the block at the caret back into a paragraph.
    SetParagraph,
    /// Wraps the block at the caret in a block quote.
    WrapInQuote,

    /// Links the selection. `None` removes the link.
    SetLink {
        #[serde(default)]
        url: Option<String>,
    },
    /// Inserts an inline image at the caret.
    InsertImage(ImageNode),
    /// Inserts an empty paragraph after the block at the caret.
    InsertParagraph,
    /// Inserts a horizontal rule after the block at the caret.
    InsertThematicBreak,

    Undo,
    Redo,
}

impl EditorCommand {
    pub fn insert_text(text: impl Into<String>) -> Self {
        EditorCommand::InsertText(text.into())
    }

    pub fn toggle_bold() -> Self {
        EditorCommand::ToggleMark(MarkSet::BOLD)
    }

    pub fn toggle_italic() -> Self {
        EditorCommand::ToggleMark(MarkSet::ITALIC)
    }

    pub fn toggle_code() -> Self {
        EditorCommand::ToggleMark(MarkSet::CODE)
    }

    /// A short name for the history entry this command produces.
    pub fn label(&self) -> &'static str {
        match self {
            EditorCommand::SetSelection(_) => "Select",
            EditorCommand::InsertText(_) => "Typing",
            EditorCommand::Delete(_) => "Delete",
            EditorCommand::SplitBlock => "Split block",
            EditorCommand::MergeBlocks => "Merge blocks",
            EditorCommand::ToggleMark(_) => "Formatting",
            EditorCommand::SetHeadingLevel(_) => "Heading",
            EditorCommand::SetCodeBlock { .. } => "Code block",
            EditorCommand::SetParagraph => "Paragraph",
            EditorCommand::WrapInQuote => "Quote",
            EditorCommand::SetLink { .. } => "Link",
            EditorCommand::InsertImage(_) => "Image",
            EditorCommand::InsertParagraph => "Paragraph",
            EditorCommand::InsertThematicBreak => "Divider",
            EditorCommand::Undo => "Undo",
            EditorCommand::Redo => "Redo",
        }
    }
}
