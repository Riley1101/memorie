//! Transactions: the only way the document ever changes.
//!
//! A command (⌘B, a keystroke, an AI rewrite) turns into one transaction
//! holding one or more operations. Applying it is atomic: every operation is
//! inverted as it goes, and a failure halfway rolls the earlier ones back, so
//! the document is never left half-edited. The inverse transaction is what
//! history stores — undo is "apply the inverse", not "reparse an old string".
//!
//! There are deliberately only three operations. Every edit — typing,
//! deleting, toggling a mark, splitting a paragraph, changing a heading level,
//! adding a table row — is one of them, and each is its own exact inverse, so
//! no operation needs a hand-written undo rule that could drift.

use crate::document::Document;
use crate::error::{EditorError, Result};
use crate::ids::BlockId;
use crate::inline::InlineContent;
use crate::node::{Block, BlockKind};
use crate::selection::SelectionRange;
use serde::{Deserialize, Serialize};

/// Byte bounds for a character range, clamped to the string.
fn char_bounds(text: &str, start: usize, end: usize) -> (usize, usize) {
    let mut offsets = text.char_indices().map(|(at, _)| at).collect::<Vec<_>>();
    offsets.push(text.len());
    let last = offsets.len() - 1;
    (
        offsets[start.min(last)],
        offsets[end.min(last).max(start.min(last))],
    )
}

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
    /// inserting and removing.
    ReplaceBlocks {
        start: usize,
        end: usize,
        blocks: Vec<Block>,
    },
    /// Replaces `start..end` of a code block's text, in characters. Code is
    /// text, not inline content — nothing inside it is marked up — so it needs
    /// its own operation rather than pretending to have runs.
    ReplaceCode {
        block: BlockId,
        start: usize,
        end: usize,
        text: String,
    },
    /// Replaces one block, wherever it sits — top level, or nested in a quote,
    /// list item or table cell.
    ///
    /// This is how a container is restructured: to add a table row or flip a
    /// list from bullets to numbers, the whole container block is replaced. The
    /// blocks inside it keep their ids, so a caret in a table cell survives a
    /// row being inserted above it.
    ReplaceBlock { block: BlockId, with: Box<Block> },
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
            Operation::ReplaceCode {
                block,
                start,
                end,
                text,
            } => {
                let target = document
                    .block_mut(block)
                    .ok_or(EditorError::NoSuchBlock(block))?;
                let BlockKind::CodeBlock { code, .. } = &mut target.kind else {
                    return Err(EditorError::NotTextual(block));
                };
                let (from, to) = char_bounds(code, start, end);
                let removed = code[from..to].to_string();
                let new_len = text.chars().count();
                code.replace_range(from..to, &text);
                Ok(Operation::ReplaceCode {
                    block,
                    start,
                    end: start + new_len,
                    text: removed,
                })
            }
            Operation::ReplaceBlock { block, with } => {
                let id = with.id;
                let removed = document
                    .replace_block(block, *with)
                    .ok_or(EditorError::NoSuchBlock(block))?;
                Ok(Operation::ReplaceBlock {
                    block: id,
                    with: Box::new(removed),
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

/// What a transaction touched, so a host can redraw that much and no more.
///
/// A keystroke changes one block's content; pressing return changes the block
/// list. The first is most of what an editor does, and it is the case worth
/// keeping cheap — sending a whole document over IPC for every character is
/// the difference between typing that feels native and typing that doesn't.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    /// Blocks whose content changed, including a container whose subtree did.
    pub blocks: Vec<BlockId>,
    /// Set when the top-level block list itself changed — blocks added, removed
    /// or reordered — so their order has to be sent again.
    pub structural: bool,
}

impl Change {
    fn touch(&mut self, block: BlockId) {
        if !self.blocks.contains(&block) {
            self.blocks.push(block);
        }
    }

    /// Folds another change into this one, for a batch applied as a unit.
    pub fn merge(&mut self, other: Change) {
        self.structural |= other.structural;
        for block in other.blocks {
            self.touch(block);
        }
    }
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

    /// What this transaction touches.
    pub fn change(&self) -> Change {
        let mut change = Change::default();
        for operation in &self.operations {
            match operation {
                Operation::ReplaceInline { block, .. } => change.touch(*block),
                Operation::ReplaceCode { block, .. } => change.touch(*block),
                Operation::ReplaceBlock { block, with } => {
                    change.touch(*block);
                    change.touch(with.id);
                }
                Operation::ReplaceBlocks { .. } => change.structural = true,
            }
        }
        change
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
