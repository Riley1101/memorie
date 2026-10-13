//! Fragments: a piece of a document on its way out to a clipboard, or in from
//! one.
//!
//! A copy is not a document. What the writer selected may be a run of
//! characters from the middle of a sentence, the raw text of half a line of
//! code, or four blocks spanning two list items — and the three cannot be
//! serialized the same way. The enum is the distinction, so nothing downstream
//! has to guess from a `Vec<Block>` whether its single paragraph was a whole
//! paragraph or the middle of one.
//!
//! Markdown is the clipboard format for the same reason it is the storage
//! format: it round-trips through this crate, and it is what another editor
//! will make the most sense of.

use crate::inline::InlineContent;
use crate::markdown;
use crate::node::Block;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "fragment", rename_all = "camelCase")]
pub enum Fragment {
    /// A run of inline content from inside one block. Pasting it puts
    /// characters in without disturbing the block they land in.
    Inline { content: InlineContent },
    /// Raw text from inside a code block. Nothing in it is marked up, and its
    /// Markdown is itself: escaping it would paste the escapes.
    Text { text: String },
    /// Whole blocks, trimmed at the ends to what the selection covered.
    Blocks { blocks: Vec<Block> },
}

impl Fragment {
    pub fn is_empty(&self) -> bool {
        match self {
            Fragment::Inline { content } => content.is_empty(),
            Fragment::Text { text } => text.is_empty(),
            Fragment::Blocks { blocks } => blocks.is_empty(),
        }
    }

    /// The fragment as Markdown — what belongs on a clipboard, and what a
    /// paste parses back.
    pub fn to_markdown(&self) -> String {
        match self {
            Fragment::Inline { content } => markdown::inline_to_markdown(content),
            Fragment::Text { text } => text.clone(),
            Fragment::Blocks { blocks } => {
                let out = markdown::blocks_to_markdown(blocks, 0);
                out.trim_end_matches('\n').to_string()
            }
        }
    }

    /// The fragment as plain text, for a clipboard reader that wants no syntax
    /// — a terminal, a search field, another app's plain-text target.
    pub fn to_plain_text(&self) -> String {
        match self {
            Fragment::Inline { content } => content.plain_text(),
            Fragment::Text { text } => text.clone(),
            Fragment::Blocks { blocks } => {
                let mut out = String::new();
                for block in blocks {
                    push_text(block, &mut out);
                }
                while out.ends_with('\n') {
                    out.pop();
                }
                out
            }
        }
    }
}

/// A block's text and its children's, blocks separated by a blank line — the
/// same shape [`crate::Document::plain_text`] produces, so a copy of a whole
/// document reads like the document.
fn push_text(block: &Block, out: &mut String) {
    let own = block.own_text();
    if !own.is_empty() {
        out.push_str(&own);
        out.push_str("\n\n");
    }
    for child in block.children() {
        push_text(child, out);
    }
}
