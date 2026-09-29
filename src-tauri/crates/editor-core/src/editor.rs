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
use crate::node::{Block, BlockKind, ColumnAlignment, ListItem, TableCell, TableNode, TableRow};
use crate::selection::{Position, SelectionRange};
use crate::transaction::{Change, Operation, Transaction};

/// How close together two edits have to be to become one undo step.
const DEFAULT_COALESCE_WINDOW_MS: i64 = 700;

pub struct Editor {
    document: Document,
    selection: SelectionRange,
    history: History,
    /// Marks the next typed text will carry, when they differ from the marks
    /// already at the caret. This is caret state, not document state: ⌘B with
    /// nothing selected changes it, and nothing else.
    pending_marks: Option<MarkSet>,
    coalesce_window_ms: i64,
}

impl Editor {
    /// Opens a document with the caret at the start of its first text block.
    pub fn new(document: Document) -> Self {
        let selection = SelectionRange::collapsed(start_of_document(&document));
        Self {
            document,
            selection,
            history: History::new(),
            pending_marks: None,
            coalesce_window_ms: DEFAULT_COALESCE_WINDOW_MS,
        }
    }

    /// How long a pause breaks a run of typing into a second undo step.
    /// Coalescing only happens when the host stamps its commands with a time
    /// (see [`Editor::apply_at`]).
    pub fn set_coalesce_window_ms(&mut self, window: i64) {
        self.coalesce_window_ms = window;
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

    /// Marks set for text not yet typed, if any.
    pub fn pending_marks(&self) -> Option<MarkSet> {
        self.pending_marks
    }

    /// What a formatting toolbar should show as active: the marks the selection
    /// carries throughout, or the ones waiting to be typed.
    pub fn active_marks(&self) -> MarkSet {
        if let Some(pending) = self.pending_marks {
            return pending;
        }
        match self.selection.single_block_range() {
            Some((block, start, end)) => {
                let Some(content) = self.document.block(block).and_then(Block::content) else {
                    return MarkSet::NONE;
                };
                if start == end {
                    return content.marks_at(start);
                }
                [
                    MarkSet::BOLD,
                    MarkSet::ITALIC,
                    MarkSet::CODE,
                    MarkSet::STRIKE,
                ]
                .into_iter()
                .filter(|mark| content.has_mark_throughout(start, end, *mark))
                .fold(MarkSet::NONE, MarkSet::with)
            }
            None => MarkSet::NONE,
        }
    }

    /// Applies a command, returning what it changed. See [`Editor::apply_at`]
    /// to stamp the history entry.
    pub fn apply(&mut self, command: EditorCommand) -> Result<Change> {
        self.apply_at(command, None)
    }

    /// Applies a command, stamping its history entry with `created_at`
    /// (milliseconds since the epoch). The host supplies the clock so the
    /// engine stays deterministic under test.
    pub fn apply_at(&mut self, command: EditorCommand, created_at: Option<i64>) -> Result<Change> {
        match command {
            // Moving the caret is not an edit: it changes no content, so it
            // neither bumps the revision nor fills the history with noise.
            EditorCommand::SetSelection(range) => {
                self.selection = range.clamp(&self.document, start_of_document(&self.document));
                // Marks waiting to be typed belong to where the caret was.
                self.pending_marks = None;
                Ok(Change::default())
            }
            // With nothing selected there is no text to format yet, so ⌘B sets
            // what the next keystroke will carry instead of editing anything.
            EditorCommand::ToggleMark(mark) if self.selection.is_collapsed() => {
                let now = self.active_marks();
                self.pending_marks = Some(if now.contains(mark) {
                    now.without(mark)
                } else {
                    now.with(mark)
                });
                Ok(Change::default())
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
            EditorCommand::SetBlockText(text) => self.set_block_text(text),
            EditorCommand::SplitBlock => self.split_block(),
            EditorCommand::MergeBlocks => self.merge_blocks(),
            EditorCommand::ToggleMark(mark) => self.toggle_mark(mark),
            EditorCommand::SetHeadingLevel(level) => self.set_heading_level(level),
            EditorCommand::SetParagraph => self.set_heading_level(0),
            EditorCommand::SetCodeBlock { language } => self.set_code_block(language),
            EditorCommand::WrapInQuote => self.wrap_in_quote(),
            EditorCommand::ToggleList { ordered } => self.toggle_list(ordered),
            EditorCommand::ToggleTask => self.toggle_task(),
            EditorCommand::InsertTable { rows, columns } => self.insert_table(rows, columns),
            EditorCommand::InsertTableRow { before } => self.insert_table_row(before),
            EditorCommand::InsertTableColumn { before } => self.insert_table_column(before),
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
    ) -> Result<Change> {
        let inverse = transaction.apply(&mut self.document)?;
        let change = transaction.change();
        self.selection = transaction
            .selection_after
            .clamp(&self.document, start_of_document(&self.document));
        self.pending_marks = None;

        // A burst of typing is one undo step. Nothing else coalesces: undoing
        // half a table insertion would be worse than an extra press.
        let coalescing = label == EditorCommand::InsertText(String::new()).label();
        if coalescing
            && self.history.coalesce(
                inverse.clone(),
                transaction.clone(),
                Some(label),
                created_at,
                self.coalesce_window_ms,
            )
        {
            return Ok(change);
        }
        self.history
            .record(inverse, transaction, Some(label.to_string()), created_at);
        Ok(change)
    }

    fn step_history(&mut self, undo: bool) -> Result<Change> {
        let transaction = if undo {
            self.history.undo()
        } else {
            self.history.redo()
        };
        let transaction = transaction.ok_or(EditorError::NothingToDo)?;
        transaction.apply(&mut self.document)?;
        self.pending_marks = None;
        self.selection = transaction
            .selection_after
            .clamp(&self.document, start_of_document(&self.document));
        Ok(transaction.change())
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

    /// Whether the caret's block holds runs or code, so the commands that edit
    /// text can serve both without pretending code has runs.
    fn caret_holds_code(&self) -> bool {
        self.document
            .block(self.selection.head.block)
            .map(|block| block.kind.code().is_some())
            .unwrap_or(false)
    }

    /// The selected range inside a code block, as characters of its code.
    fn code_target(&self) -> Result<(BlockId, usize, usize)> {
        let (block, start, end) = self
            .selection
            .single_block_range()
            .ok_or(EditorError::MultiBlockSelection)?;
        let len = self.document.block_len(block);
        Ok((block, start.min(len), end.min(len)))
    }

    /// A block's inline content, or an error naming why it has none.
    fn content_of(&self, block: BlockId) -> Result<&InlineContent> {
        self.document
            .block(block)
            .ok_or(EditorError::NoSuchBlock(block))?
            .content()
            .ok_or(EditorError::NotTextual(block))
    }

    /// The operations that put `content` where the selection is, and where the
    /// caret ends up.
    ///
    /// A selection spanning blocks is served by joining its two ends: what
    /// follows the selection in the last block moves onto the first block, and
    /// every block between them (the last included) is removed. Both ends have
    /// to be top-level blocks — a selection ending inside a quote or a list is
    /// refused rather than half-handled.
    fn replace_selection_ops(&self, content: InlineContent) -> Result<(Vec<Operation>, Position)> {
        if let Some((block, start, end)) = self.selection.single_block_range() {
            self.content_of(block)?;
            let caret = Position::new(block, start + content.len());
            return Ok((
                vec![Operation::ReplaceInline {
                    block,
                    start,
                    end,
                    content,
                }],
                caret,
            ));
        }

        let (from, to) = self.selection.ordered(&self.document);
        let first = self.top_index(from.block)?;
        let last = self.top_index(to.block)?;
        if last <= first {
            return Err(EditorError::Unsupported(
                "a selection whose ends are not in document order",
            ));
        }
        self.content_of(from.block)?;

        // What survives in the last block joins the first one, right after the
        // inserted content.
        let tail = self
            .document
            .block(to.block)
            .and_then(Block::content)
            .map(|content| content.slice(to.offset, content.len()))
            .unwrap_or_default();
        let caret = Position::new(from.block, from.offset + content.len());
        let mut joined: Vec<Inline> = content.pieces().to_vec();
        joined.extend(tail.pieces().iter().cloned());

        Ok((
            vec![
                Operation::ReplaceInline {
                    block: from.block,
                    start: from.offset,
                    end: self.document.block_len(from.block),
                    content: InlineContent::new(joined),
                },
                Operation::ReplaceBlocks {
                    start: first + 1,
                    end: last + 1,
                    blocks: Vec::new(),
                },
            ],
            caret,
        ))
    }

    /// Types `text` into a code block, where there is no formatting to carry.
    fn insert_code(&self, text: String) -> Result<Transaction> {
        let (block, start, end) = self.code_target()?;
        let caret = Position::new(block, start + text.chars().count());
        Ok(
            Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                Operation::ReplaceCode {
                    block,
                    start,
                    end,
                    text,
                },
            ),
        )
    }

    /// Replaces the selection with `content`, or with `text` carrying the marks
    /// and link in force at the insertion point when `content` is empty.
    fn insert_content(&self, content: InlineContent, text: String) -> Result<Transaction> {
        if self.caret_holds_code() && content.is_empty() {
            return self.insert_code(text);
        }
        let content = if content.is_empty() {
            if text.is_empty() {
                return Err(EditorError::Unsupported("inserting nothing"));
            }
            let (start, _) = self.selection.ordered(&self.document);
            let existing = self.content_of(start.block)?;
            InlineContent::new(vec![Inline::Text {
                text,
                marks: self
                    .pending_marks
                    .unwrap_or_else(|| existing.marks_at(start.offset)),
                link: existing.link_spanning(start.offset).cloned(),
            }])
        } else {
            content
        };

        let (operations, caret) = self.replace_selection_ops(content)?;
        let mut transaction = Transaction::new(self.selection, SelectionRange::collapsed(caret));
        transaction.operations = operations;
        Ok(transaction)
    }

    fn delete(&self, direction: Direction) -> Result<Transaction> {
        if self.caret_holds_code() {
            let (block, start, end) = self.code_target()?;
            let (start, end) = match (start == end, direction) {
                (false, _) => (start, end),
                (true, Direction::Backward) if start > 0 => (start - 1, start),
                (true, Direction::Forward) if end < self.document.block_len(block) => {
                    (start, end + 1)
                }
                // At either edge of a code block there is nothing to join to:
                // merging code into prose would lose the fence.
                (true, _) => return Err(EditorError::NothingToDo),
            };
            let caret = Position::new(block, start);
            return Ok(
                Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                    Operation::ReplaceCode {
                        block,
                        start,
                        end,
                        text: String::new(),
                    },
                ),
            );
        }

        if !self.selection.is_collapsed() {
            let (operations, caret) = self.replace_selection_ops(InlineContent::empty())?;
            let mut transaction =
                Transaction::new(self.selection, SelectionRange::collapsed(caret));
            transaction.operations = operations;
            return Ok(transaction);
        }

        let (block, start, _, content) = self.inline_target()?;
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

    /// The smallest edit that turns the caret block's text into `text`.
    ///
    /// The common prefix and suffix are left untouched, so only what actually
    /// changed is replaced and formatting on either side of it survives. New
    /// text takes the marks in force where it starts, so composing inside a
    /// bold word stays bold.
    fn set_block_text(&self, text: String) -> Result<Transaction> {
        let block = self.selection.head.block;
        let before_text = self
            .document
            .offset_text(block)
            .ok_or(EditorError::NotTextual(block))?;

        let before: Vec<char> = before_text.chars().collect();
        let after: Vec<char> = text.chars().collect();

        let start = before
            .iter()
            .zip(&after)
            .take_while(|(a, b)| a == b)
            .count();
        let tail = before
            .iter()
            .rev()
            .zip(after.iter().rev())
            .take(before.len().min(after.len()) - start)
            .take_while(|(a, b)| a == b)
            .count();
        let end = before.len() - tail;
        let inserted: String = after[start..after.len() - tail].iter().collect();

        if start == end && inserted.is_empty() {
            return Err(EditorError::NothingToDo);
        }

        // Code has no runs to carry formatting, so the edit is the text itself.
        if self.caret_holds_code() {
            let caret = Position::new(block, start + inserted.chars().count());
            return Ok(
                Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                    Operation::ReplaceCode {
                        block,
                        start,
                        end,
                        text: inserted,
                    },
                ),
            );
        }
        let content = self.content_of(block)?;

        // Replacing text takes the formatting of the text it replaces, so an
        // IME composing over a bold word stays bold. Pure insertion takes the
        // formatting in force at the caret instead — the trailing-edge rule,
        // which `marks_at` implements.
        let sample = if end > start {
            (start + 1).min(end)
        } else {
            start
        };
        let replacement = if inserted.is_empty() {
            InlineContent::empty()
        } else {
            InlineContent::new(vec![Inline::Text {
                text: inserted,
                marks: self
                    .pending_marks
                    .unwrap_or_else(|| content.marks_at(sample)),
                link: content.link_spanning(sample).cloned(),
            }])
        };

        let caret = Position::new(block, start + replacement.len());
        Ok(
            Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(
                Operation::ReplaceInline {
                    block,
                    start,
                    end,
                    content: replacement,
                },
            ),
        )
    }

    fn split_block(&mut self) -> Result<Transaction> {
        // Return inside code is a new line of code, not a new block: splitting
        // would leave two fences where the writer wanted one.
        if self.caret_holds_code() {
            return self.insert_code("\n".to_string());
        }
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

    fn toggle_list(&mut self, ordered: bool) -> Result<Transaction> {
        let block = self.selection.head.block;

        if let Some((list_id, _)) = self.document.list_position(block) {
            let BlockKind::List {
                ordered: was,
                start,
                tight,
                items,
            } = self
                .document
                .block(list_id)
                .expect("found above")
                .kind
                .clone()
            else {
                unreachable!("list_position only points at lists");
            };

            // The same kind again means "stop being a list": the items' blocks
            // take the list's place, keeping their ids and the caret with them.
            if was == ordered {
                let index = self.top_index(list_id)?;
                let blocks: Vec<Block> = items.into_iter().flat_map(|item| item.blocks).collect();
                return Ok(Transaction::new(self.selection, self.selection).with(
                    Operation::ReplaceBlocks {
                        start: index,
                        end: index + 1,
                        blocks,
                    },
                ));
            }

            // Bullets to numbers or back. A nested list is replaced in place,
            // which is what ReplaceBlock is for.
            let flipped = Block::new(
                list_id,
                BlockKind::List {
                    ordered,
                    start: start.filter(|_| ordered),
                    tight,
                    items,
                },
            );
            return Ok(Transaction::new(self.selection, self.selection).with(
                Operation::ReplaceBlock {
                    block: list_id,
                    with: Box::new(flipped),
                },
            ));
        }

        let index = self.top_index(block)?;
        let inner = self.document.block(block).expect("checked above").clone();
        let list = self.document.new_block(BlockKind::List {
            ordered,
            start: None,
            tight: true,
            items: vec![ListItem::new(vec![inner])],
        });
        Ok(
            Transaction::new(self.selection, self.selection).with(Operation::ReplaceBlocks {
                start: index,
                end: index + 1,
                blocks: vec![list],
            }),
        )
    }

    fn toggle_task(&self) -> Result<Transaction> {
        let (list_id, item) = self
            .document
            .list_position(self.selection.head.block)
            .ok_or(EditorError::Unsupported("the caret is not in a list"))?;
        let mut list = self.document.block(list_id).expect("found above").clone();
        let BlockKind::List { items, .. } = &mut list.kind else {
            unreachable!("list_position only points at lists");
        };
        // A plain item becomes an unticked task; a task is ticked and unticked.
        items[item].checked = match items[item].checked {
            None => Some(false),
            Some(checked) => Some(!checked),
        };

        Ok(
            Transaction::new(self.selection, self.selection).with(Operation::ReplaceBlock {
                block: list_id,
                with: Box::new(list),
            }),
        )
    }

    fn insert_table(&mut self, rows: usize, columns: usize) -> Result<Transaction> {
        let block = self.selection.head.block;
        let index = self.top_index(block)?;
        let columns = columns.max(1);

        // A GFM table is a header row plus body rows; `rows` counts the body.
        let mut table_rows = Vec::with_capacity(rows + 1);
        for _ in 0..rows + 1 {
            table_rows.push(TableRow {
                cells: (0..columns).map(|_| self.empty_cell()).collect(),
            });
        }
        let caret = table_rows[0].cells[0].blocks[0].id;
        let table = self.document.new_block(BlockKind::Table {
            table: TableNode {
                alignments: vec![ColumnAlignment::None; columns],
                rows: table_rows,
            },
        });

        Ok(Transaction::new(
            self.selection,
            SelectionRange::collapsed(Position::new(caret, 0)),
        )
        .with(Operation::ReplaceBlocks {
            start: index + 1,
            end: index + 1,
            blocks: vec![table],
        }))
    }

    fn insert_table_row(&mut self, before: bool) -> Result<Transaction> {
        let (table_id, row, _) = self.table_position()?;
        let mut block = self.document.block(table_id).expect("found above").clone();
        let BlockKind::Table { table } = &mut block.kind else {
            unreachable!("table_position only points at tables");
        };

        let columns = table
            .rows
            .iter()
            .map(|row| row.cells.len())
            .max()
            .unwrap_or(1);
        let cells: Vec<TableCell> = (0..columns).map(|_| self.empty_cell()).collect();
        let caret = cells[0].blocks[0].id;
        // The header row stays the header: a row inserted "before" the header
        // goes under it.
        let at = (row + usize::from(!before)).max(1);
        table
            .rows
            .insert(at.min(table.rows.len()), TableRow { cells });

        Ok(Transaction::new(
            self.selection,
            SelectionRange::collapsed(Position::new(caret, 0)),
        )
        .with(Operation::ReplaceBlock {
            block: table_id,
            with: Box::new(block),
        }))
    }

    fn insert_table_column(&mut self, before: bool) -> Result<Transaction> {
        let (table_id, row, column) = self.table_position()?;
        let at = column + usize::from(!before);
        let mut block = self.document.block(table_id).expect("found above").clone();

        let rows = match &block.kind {
            BlockKind::Table { table } => table.rows.len(),
            _ => unreachable!("table_position only points at tables"),
        };
        let cells: Vec<TableCell> = (0..rows).map(|_| self.empty_cell()).collect();
        let caret = cells.get(row).map(|cell| cell.blocks[0].id);

        let BlockKind::Table { table } = &mut block.kind else {
            unreachable!("checked above");
        };
        for (index, cell) in cells.into_iter().enumerate() {
            let row = &mut table.rows[index];
            let at = at.min(row.cells.len());
            row.cells.insert(at, cell);
        }
        if table.alignments.len() < table.rows[0].cells.len() {
            table
                .alignments
                .insert(at.min(table.alignments.len()), ColumnAlignment::None);
        }

        let after = caret
            .map(|caret| SelectionRange::collapsed(Position::new(caret, 0)))
            .unwrap_or(self.selection);
        Ok(
            Transaction::new(self.selection, after).with(Operation::ReplaceBlock {
                block: table_id,
                with: Box::new(block),
            }),
        )
    }

    /// The table the caret is in, with its row and column.
    fn table_position(&self) -> Result<(BlockId, usize, usize)> {
        self.document
            .table_position(self.selection.head.block)
            .ok_or(EditorError::Unsupported("the caret is not in a table"))
    }

    /// A table cell holding one empty paragraph, which is where a caret can go.
    fn empty_cell(&mut self) -> TableCell {
        TableCell {
            blocks: vec![self.document.new_block(BlockKind::paragraph(""))],
        }
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
