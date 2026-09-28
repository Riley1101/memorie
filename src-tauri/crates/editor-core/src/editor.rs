//! The editor: a document, a selection, and a history, with commands as the
//! only door in.
//!
//! Every command turns into a [`Transaction`], which is applied atomically and
//! recorded with its inverse. Nothing here knows about Markdown, files, Tauri
//! or the frontend.

use crate::command::{Direction, EditorCommand};
use crate::document::Document;
use crate::error::{EditorError, Result};
use crate::history::History;
use crate::ids::BlockId;
use crate::inline::{Inline, InlineContent, Link, MarkSet};
use crate::node::{Block, BlockKind};
use crate::selection::{Position, SelectionRange};
use crate::transaction::{Operation, Transaction};

pub struct Editor {
    document: Document,
    selection: SelectionRange,
    history: History,
}

impl Editor {
    /// Opens a document with the caret at the start of its first text block.
    pub fn new(document: Document) -> Self {
        let selection = SelectionRange::collapsed(start_of_document(&document));
        Self {
            document,
            selection,
            history: History::new(),
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn selection(&self) -> SelectionRange {
        self.selection
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn revision(&self) -> u64 {
        self.document.revision()
    }

    /// Applies a command. See [`Editor::apply_at`] to stamp the history entry.
    pub fn apply(&mut self, command: EditorCommand) -> Result<()> {
        self.apply_at(command, None)
    }

    /// Applies a command, stamping its history entry with `created_at`
    /// (milliseconds since the epoch). The host supplies the clock so the
    /// engine stays deterministic under test.
    pub fn apply_at(&mut self, command: EditorCommand, created_at: Option<i64>) -> Result<()> {
        match command {
            // Moving the caret is not an edit: it changes no content, so it
            // neither bumps the revision nor fills the history with noise.
            EditorCommand::SetSelection(range) => {
                self.selection = range.clamp(&self.document, start_of_document(&self.document));
                Ok(())
            }
            EditorCommand::Undo => self.step_history(true),
            EditorCommand::Redo => self.step_history(false),
            other => {
                let label = other.label();
                let transaction = self.build(other)?;
                self.commit(transaction, label, created_at)
            }
        }
    }

    /// Turns a command into the transaction that carries it out.
    ///
    /// Takes `&mut self` only to hand out block ids: ids are monotonic and
    /// never reused, so one being spent by a transaction that then fails is
    /// harmless.
    fn build(&mut self, command: EditorCommand) -> Result<Transaction> {
        match command {
            EditorCommand::InsertText(text) => self.insert_content(InlineContent::empty(), text),
            EditorCommand::InsertImage(image) => self.insert_content(
                InlineContent::new(vec![Inline::Image(image)]),
                String::new(),
            ),
            EditorCommand::Delete(direction) => self.delete(direction),
            EditorCommand::SplitBlock => self.split_block(),
            EditorCommand::MergeBlocks => self.merge_blocks(),
            EditorCommand::ToggleMark(mark) => self.toggle_mark(mark),
            EditorCommand::SetHeadingLevel(level) => self.set_heading_level(level),
            EditorCommand::SetParagraph => self.set_heading_level(0),
            EditorCommand::SetCodeBlock { language } => self.set_code_block(language),
            EditorCommand::WrapInQuote => self.wrap_in_quote(),
            EditorCommand::SetLink { url } => self.set_link(url),
            EditorCommand::InsertParagraph => {
                self.insert_block_after_caret(BlockKind::paragraph(""), true)
            }
            EditorCommand::InsertThematicBreak => {
                self.insert_block_after_caret(BlockKind::ThematicBreak, false)
            }
            EditorCommand::SetSelection(_) | EditorCommand::Undo | EditorCommand::Redo => {
                Err(EditorError::Unsupported("handled before build"))
            }
        }
    }

    fn commit(
        &mut self,
        transaction: Transaction,
        label: &str,
        created_at: Option<i64>,
    ) -> Result<()> {
        let inverse = transaction.apply(&mut self.document)?;
        self.selection = transaction
            .selection_after
            .clamp(&self.document, start_of_document(&self.document));
        self.history
            .record(inverse, transaction, Some(label.to_string()), created_at);
        Ok(())
    }

    fn step_history(&mut self, undo: bool) -> Result<()> {
        let transaction = if undo {
            self.history.undo()
        } else {
            self.history.redo()
        };
        let transaction = transaction.ok_or(EditorError::NothingToDo)?;
        transaction.apply(&mut self.document)?;
        self.selection = transaction
            .selection_after
            .clamp(&self.document, start_of_document(&self.document));
        Ok(())
    }

    // --- command builders ---------------------------------------------------

    /// The selection as a range inside one block, with that block's content.
    fn inline_target(&self) -> Result<(BlockId, usize, usize, &InlineContent)> {
        let (block, start, end) = self
            .selection
            .single_block_range()
            .ok_or(EditorError::MultiBlockSelection)?;
        let content = self
            .document
            .block(block)
            .ok_or(EditorError::NoSuchBlock(block))?
            .content()
            .ok_or(EditorError::NotTextual(block))?;
        Ok((block, start, end, content))
    }

    /// Same, with the content cloned, for the builders that also need to hand
    /// out block ids and so can't hold a borrow of the document.
    fn inline_target_owned(&self) -> Result<(BlockId, usize, usize, InlineContent)> {
        let (block, start, end, content) = self.inline_target()?;
        Ok((block, start, end, content.clone()))
    }

    /// Replaces the selection with `content`, or with `text` carrying the marks
    /// and link in force at the insertion point when `content` is empty.
    fn insert_content(&self, content: InlineContent, text: String) -> Result<Transaction> {
        let (block, start, end, existing) = self.inline_target()?;
        let content = if content.is_empty() {
            if text.is_empty() {
                return Err(EditorError::Unsupported("inserting nothing"));
            }
            InlineContent::new(vec![Inline::Text {
                text,
                marks: existing.marks_at(start),
                link: existing.link_spanning(start).cloned(),
            }])
        } else {
            content
        };

        let caret = Position::new(block, start + content.len());
        Ok(
            Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                Operation::ReplaceInline {
                    block,
                    start,
                    end,
                    content,
                },
            ),
        )
    }

    fn delete(&self, direction: Direction) -> Result<Transaction> {
        let (block, start, end, content) = self.inline_target()?;

        if start != end {
            let caret = Position::new(block, start);
            return Ok(
                Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                    Operation::ReplaceInline {
                        block,
                        start,
                        end,
                        content: InlineContent::empty(),
                    },
                ),
            );
        }

        match direction {
            Direction::Backward if start > 0 => {
                let caret = Position::new(block, start - 1);
                Ok(
                    Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                        Operation::ReplaceInline {
                            block,
                            start: start - 1,
                            end: start,
                            content: InlineContent::empty(),
                        },
                    ),
                )
            }
            Direction::Forward if start < content.len() => Ok(Transaction::new(
                self.selection,
                SelectionRange::collapsed(Position::new(block, start)),
            )
            .with(Operation::ReplaceInline {
                block,
                start,
                end: start + 1,
                content: InlineContent::empty(),
            })),
            // At a block edge, a delete joins two blocks instead.
            Direction::Backward => self.merge(block),
            Direction::Forward => {
                let next = self
                    .document
                    .text_block_after(block)
                    .ok_or(EditorError::NothingToDo)?;
                self.merge(next)
            }
        }
    }

    fn split_block(&mut self) -> Result<Transaction> {
        let (block, start, end, content) = self.inline_target_owned()?;
        let index = self.top_index(block)?;
        let heading_level = match &self.document.block(block).expect("checked above").kind {
            BlockKind::Heading { level, .. } => Some(*level),
            _ => None,
        };

        let head = content.slice(0, start);
        let tail = content.slice(end, content.len());

        // The first half keeps the block's identity — and with it any
        // annotation or cursor that points at it.
        let first = Block::new(
            block,
            match heading_level {
                Some(level) => BlockKind::Heading {
                    level,
                    content: head,
                },
                None => BlockKind::Paragraph { content: head },
            },
        );
        // Splitting a heading leaves prose behind it, never a second heading.
        let second = self
            .document
            .new_block(BlockKind::Paragraph { content: tail });

        let caret = Position::new(second.id, 0);
        Ok(
            Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                Operation::ReplaceBlocks {
                    start: index,
                    end: index + 1,
                    blocks: vec![first, second],
                },
            ),
        )
    }

    fn merge_blocks(&mut self) -> Result<Transaction> {
        let (block, _, _, _) = self.inline_target()?;
        self.merge(block)
    }

    /// Joins `block` onto the text block before it.
    fn merge(&self, block: BlockId) -> Result<Transaction> {
        let previous = self
            .document
            .text_block_before(block)
            .ok_or(EditorError::NothingToDo)?;
        let index = self.top_index(block)?;
        let previous_index = self.top_index(previous)?;
        if previous_index + 1 != index {
            return Err(EditorError::Unsupported("merging across nested blocks"));
        }

        let tail = self
            .document
            .block(block)
            .and_then(Block::content)
            .cloned()
            .ok_or(EditorError::NotTextual(block))?;
        let joint = self.document.block_len(previous);

        let caret = Position::new(previous, joint);
        Ok(
            Transaction::new(self.selection, SelectionRange::collapsed(caret))
                .with(Operation::ReplaceInline {
                    block: previous,
                    start: joint,
                    end: joint,
                    content: tail,
                })
                .with(Operation::ReplaceBlocks {
                    start: index,
                    end: index + 1,
                    blocks: Vec::new(),
                }),
        )
    }

    fn toggle_mark(&self, mark: MarkSet) -> Result<Transaction> {
        let (block, start, end, content) = self.inline_target()?;
        if start == end {
            // Marks for text not yet typed are editor state, not document
            // state; they belong with the caret (Phase 4), not here.
            return Err(EditorError::Unsupported(
                "toggling a mark with no selection",
            ));
        }

        let on = !content.has_mark_throughout(start, end, mark);
        let mut middle = content.slice(start, end);
        if on {
            middle.apply_marks(0, middle.len(), mark, MarkSet::NONE);
        } else {
            middle.apply_marks(0, middle.len(), MarkSet::NONE, mark);
        }

        Ok(
            Transaction::new(self.selection, self.selection).with(Operation::ReplaceInline {
                block,
                start,
                end,
                content: middle,
            }),
        )
    }

    fn set_link(&self, url: Option<String>) -> Result<Transaction> {
        let (block, start, end, content) = self.inline_target()?;
        if start == end {
            return Err(EditorError::Unsupported("linking an empty selection"));
        }
        let mut middle = content.slice(start, end);
        middle.apply_link(0, middle.len(), url.map(Link::new));

        Ok(
            Transaction::new(self.selection, self.selection).with(Operation::ReplaceInline {
                block,
                start,
                end,
                content: middle,
            }),
        )
    }

    fn set_heading_level(&self, level: u8) -> Result<Transaction> {
        let block = self.selection.head.block;
        let index = self.top_index(block)?;
        let existing = self.document.block(block).expect("checked above");

        let content = match &existing.kind {
            BlockKind::Paragraph { content } | BlockKind::Heading { content, .. } => {
                content.clone()
            }
            BlockKind::CodeBlock { code, .. } => InlineContent::from_text(code.clone()),
            _ => return Err(EditorError::NotTextual(block)),
        };
        let kind = if level == 0 {
            BlockKind::Paragraph { content }
        } else {
            BlockKind::heading(level, content)
        };

        Ok(
            Transaction::new(self.selection, self.selection).with(Operation::ReplaceBlocks {
                start: index,
                end: index + 1,
                blocks: vec![Block::new(block, kind)],
            }),
        )
    }

    fn set_code_block(&self, language: Option<String>) -> Result<Transaction> {
        let block = self.selection.head.block;
        let index = self.top_index(block)?;
        let existing = self.document.block(block).expect("checked above");
        let code = match &existing.kind {
            BlockKind::Paragraph { content } | BlockKind::Heading { content, .. } => {
                content.plain_text()
            }
            BlockKind::CodeBlock { code, .. } => code.clone(),
            _ => return Err(EditorError::NotTextual(block)),
        };

        let caret = Position::new(block, 0);
        Ok(
            Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                Operation::ReplaceBlocks {
                    start: index,
                    end: index + 1,
                    blocks: vec![Block::new(block, BlockKind::CodeBlock { language, code })],
                },
            ),
        )
    }

    fn wrap_in_quote(&mut self) -> Result<Transaction> {
        let block = self.selection.head.block;
        let index = self.top_index(block)?;
        let inner = self.document.block(block).expect("checked above").clone();
        // The quoted block keeps its id, so the caret inside it stays valid.
        let quote = self.document.new_block(BlockKind::Quote {
            blocks: vec![inner],
        });

        Ok(
            Transaction::new(self.selection, self.selection).with(Operation::ReplaceBlocks {
                start: index,
                end: index + 1,
                blocks: vec![quote],
            }),
        )
    }

    fn insert_block_after_caret(
        &mut self,
        kind: BlockKind,
        move_caret: bool,
    ) -> Result<Transaction> {
        let block = self.selection.head.block;
        let index = self.top_index(block)?;
        let inserted = self.document.new_block(kind);
        let id = inserted.id;

        let after = if move_caret {
            SelectionRange::collapsed(Position::new(id, 0))
        } else {
            self.selection
        };
        Ok(
            Transaction::new(self.selection, after).with(Operation::ReplaceBlocks {
                start: index + 1,
                end: index + 1,
                blocks: vec![inserted],
            }),
        )
    }

    /// Where `block` sits in the top-level list. Structural operations are
    /// top-level only for now; inline edits work at any depth.
    fn top_index(&self, block: BlockId) -> Result<usize> {
        if self.document.block(block).is_none() {
            return Err(EditorError::NoSuchBlock(block));
        }
        self.document
            .top_index(block)
            .ok_or(EditorError::NestedBlock(block))
    }
}

/// The caret's home: the start of the first block that can hold text.
fn start_of_document(document: &Document) -> Position {
    let block = document
        .text_block_ids()
        .first()
        .copied()
        .or_else(|| document.blocks().first().map(|block| block.id))
        .unwrap_or(BlockId(0));
    Position::new(block, 0)
}
