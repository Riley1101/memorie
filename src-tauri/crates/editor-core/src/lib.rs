//! A reusable document model and editing engine for Markdown-backed writing
//! apps.
//!
//! The crate depends on nothing but `serde`: no Tauri, no UI, no filesystem.
//! Memorie is its first consumer, but nothing here knows that.
//!
//! ```text
//!   EditorCommand ──▶ Editor ──▶ Transaction ──▶ Document ──▶ History
//!                                                   │
//!                                                   ▼
//!                                       plain text / Markdown / outline
//! ```
//!
//! The rules the design follows:
//!
//! * **The document is the state.** Markdown is a storage format the engine
//!   reads and writes at the edges, never the thing being edited.
//! * **Blocks are addressed by id.** Indices go stale; ids don't.
//! * **Every change is a transaction**, applied atomically and invertible, so
//!   undo is an operation on the model rather than a saved copy of the text.
//! * **Inline content is flat.** Marks are a property of a run of characters,
//!   not a tree to walk.
//!
//! # Example
//!
//! ```
//! use editor_core::{Document, Editor, EditorCommand, BlockKind, MarkSet};
//!
//! let mut editor = Editor::new(Document::from_kinds(
//!     "notes/scene.md",
//!     vec![BlockKind::paragraph("She found the ")],
//! ));
//! let block = editor.document().blocks()[0].id;
//!
//! editor.apply(EditorCommand::SetSelection(
//!     editor_core::SelectionRange::in_block(block, 14, 14),
//! ))?;
//! editor.apply(EditorCommand::insert_text("letter"))?;
//! editor.apply(EditorCommand::SetSelection(
//!     editor_core::SelectionRange::in_block(block, 14, 20),
//! ))?;
//! editor.apply(EditorCommand::ToggleMark(MarkSet::BOLD))?;
//!
//! assert_eq!(editor.document().plain_text(), "She found the letter");
//! editor.apply(EditorCommand::Undo)?;
//! assert!(!editor.document().blocks()[0]
//!     .content()
//!     .unwrap()
//!     .has_mark_throughout(14, 20, MarkSet::BOLD));
//! # Ok::<(), editor_core::EditorError>(())
//! ```

pub mod command;
pub mod document;
pub mod editor;
pub mod error;
pub mod history;
pub mod ids;
pub mod inline;
pub mod markdown;
pub mod node;
pub mod selection;
pub mod transaction;

pub use command::{Direction, EditorCommand};
pub use document::{Document, DocumentMetadata, Frontmatter, Heading};
pub use editor::Editor;
pub use error::{EditorError, Result};
pub use history::{History, NodeIndex};
pub use ids::{BlockId, DocumentId};
pub use inline::{ImageNode, Inline, InlineContent, Link, MarkSet};
pub use markdown::{parse, to_markdown, DocumentSource};
pub use node::{Block, BlockKind, ColumnAlignment, ListItem, TableCell, TableNode, TableRow};
pub use selection::{Position, SelectionRange};
pub use transaction::{Operation, Transaction};
