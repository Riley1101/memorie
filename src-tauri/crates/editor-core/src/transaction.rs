//! Transactions: the only way the document ever changes.
//!
//! A command (⌘B, a keystroke, an AI rewrite) turns into one transaction
//! holding one or more operations. Applying it is atomic: every operation is
//! inverted as it goes, and a failure halfway rolls the earlier ones back, so
//! the document is never left half-edited. The inverse transaction is what
//! history stores — undo is "apply the inverse", not "reparse an old string".
//!
//! There are deliberately only two operations. Every edit — typing, deleting,
//! toggling a mark, splitting a paragraph, changing a heading level — is one
//! of them, and each is its own exact inverse, so no operation needs a
//! hand-written undo rule that could drift.

use crate::document::Document;
use crate::error::{EditorError, Result};
use crate::ids::BlockId;
use crate::inline::InlineContent;
use crate::node::Block;
use crate::selection::SelectionRange;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Operation {
    /// Replaces `start..end` of a block's inline content. Covers insertion
    /// (empty range), deletion (empty content), and every mark or link change
    /// (same text, different marks).
    ReplaceInline {
        block: BlockId,
        start: usize,
        end: usize,
        content: InlineContent,
    },
    /// Replaces a run of top-level blocks. Covers splitting, merging,
    /// inserting, removing, and changing a block's kind.
    ///
    /// Nested blocks (inside a quote, list item or table cell) can be edited
    /// inline by id, but not yet restructured; see `EditorError::Unsupported`.
    ReplaceBlocks {
        start: usize,
        end: usize,
        blocks: Vec<Block>,
    },
}

impl Operation {
    /// Applies the operation and returns the operation that undoes it.
    fn apply(self, document: &mut Document) -> Result<Operation> {
        match self {
            Operation::ReplaceInline {
                block,
                start,
                end,
                content,
            } => {
                let target = document
                    .block_mut(block)
                    .ok_or(EditorError::NoSuchBlock(block))?;
                let inline = target.content_mut().ok_or(EditorError::NotTextual(block))?;
                let new_len = content.len();
                let removed = inline.replace_range(start, end, content);
                Ok(Operation::ReplaceInline {
                    block,
                    start,
                    end: start + new_len,
                    content: removed,
                })
            }
            Operation::ReplaceBlocks { start, end, blocks } => {
                let list = document.blocks_mut();
                if end > list.len() || start > end {
                    return Err(EditorError::OutOfRange {
                        start,
                        end,
                        len: list.len(),
                    });
                }
                let new_len = blocks.len();
                let removed: Vec<Block> = list.splice(start..end, blocks).collect();
                Ok(Operation::ReplaceBlocks {
                    start,
                    end: start + new_len,
                    blocks: removed,
                })
            }
        }
    }
}

/// A set of operations applied as one unit, with the selection before and
/// after so undo restores the caret too.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub operations: Vec<Operation>,
    pub selection_before: SelectionRange,
    pub selection_after: SelectionRange,
}

impl Transaction {
    pub fn new(selection_before: SelectionRange, selection_after: SelectionRange) -> Self {
        Self {
            operations: Vec::new(),
            selection_before,
            selection_after,
        }
    }

    pub fn with(mut self, operation: Operation) -> Self {
        self.operations.push(operation);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Applies every operation in order. On failure the already-applied ones
    /// are undone before the error is returned, so the document is unchanged.
    /// On success the document's revision moves once, however many operations
    /// the transaction held.
    pub fn apply(&self, document: &mut Document) -> Result<Transaction> {
        let mut inverse = Vec::with_capacity(self.operations.len());
        for operation in &self.operations {
            match operation.clone().apply(document) {
                Ok(undo) => inverse.push(undo),
                Err(error) => {
                    // Roll back, newest first. These inverses were produced by
                    // the very operations that just succeeded, so they apply.
                    for undo in inverse.into_iter().rev() {
                        let _ = undo.apply(document);
                    }
                    return Err(error);
                }
            }
        }
        inverse.reverse();
        document.bump_revision();
        Ok(Transaction {
            operations: inverse,
            selection_before: self.selection_after,
            selection_after: self.selection_before,
        })
    }
}
