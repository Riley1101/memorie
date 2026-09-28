//! Errors. Kept as a plain enum with no dependency on `thiserror` so the
//! crate stays cheap to depend on; the host maps these to its own error type.

use crate::ids::BlockId;
use std::fmt;

pub type Result<T> = std::result::Result<T, EditorError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorError {
    /// The block is gone — a stale id from the frontend, or an undone edit.
    NoSuchBlock(BlockId),
    /// The block holds no editable inline content (a code block, a rule).
    NotTextual(BlockId),
    /// A block range outside the top-level list.
    OutOfRange {
        start: usize,
        end: usize,
        len: usize,
    },
    /// The selection spans blocks and the command only handles one.
    MultiBlockSelection,
    /// The command needs a top-level block and got a nested one.
    NestedBlock(BlockId),
    /// Nothing to undo or redo.
    NothingToDo,
    /// Knowingly not implemented yet, with what is missing.
    Unsupported(&'static str),
}

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditorError::NoSuchBlock(id) => write!(f, "no block with id {}", id.0),
            EditorError::NotTextual(id) => {
                write!(f, "block {} holds no editable text", id.0)
            }
            EditorError::OutOfRange { start, end, len } => {
                write!(
                    f,
                    "block range {start}..{end} is outside a document of {len} blocks"
                )
            }
            EditorError::MultiBlockSelection => {
                write!(f, "this command needs a selection inside one block")
            }
            EditorError::NestedBlock(id) => {
                write!(
                    f,
                    "block {} is nested; structural edits are top-level only",
                    id.0
                )
            }
            EditorError::NothingToDo => write!(f, "nothing to undo or redo"),
            EditorError::Unsupported(what) => write!(f, "not supported yet: {what}"),
        }
    }
}

impl std::error::Error for EditorError {}
