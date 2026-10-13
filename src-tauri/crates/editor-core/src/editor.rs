//! The editor: a document, a selection, and a history, with commands as the
//! only door in.
//!
//! Every command turns into a [`Transaction`], which is applied atomically and
//! recorded with its inverse. Nothing here knows about Markdown, files, Tauri
//! or the frontend.

use crate::command::{Direction, EditorCommand, PASTE};
use crate::document::Document;
use crate::error::{EditorError, Result};
use crate::fragment::Fragment;
use crate::history::History;
use crate::ids::{BlockId, IdGenerator};
use crate::inline::{Inline, InlineContent, Link, MarkSet};
use crate::markdown;
use crate::node::{Block, BlockKind, ColumnAlignment, ListItem, TableCell, TableNode, TableRow};
use crate::selection::{Position, SelectionRange};
use crate::transaction::{Change, Operation, Transaction};

/// How close together two edits have to be to become one undo step.
const DEFAULT_COALESCE_WINDOW_MS: i64 = 700;

/// The label the commands that type text carry; the only ones that coalesce.
const TYPING: &str = "Typing";

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

    /// What the selection holds, for a copy or a cut.
    ///
    /// Copying changes nothing, so this is not a command: the host asks for the
    /// fragment, puts [`Fragment::to_markdown`] and
    /// [`Fragment::to_plain_text`] on the clipboard, and sends
    /// [`EditorCommand::Delete`] afterwards for a cut.
    ///
    /// The three shapes a selection can have are kept apart (see
    /// [`Fragment`]): a run of characters from one block, raw text from inside
    /// a code block, and whole blocks trimmed at the ends. A selection that
    /// covers a block exactly is blocks, not inline, so pasting it back keeps
    /// the block break.
    pub fn copy(&self) -> Result<Fragment> {
        if self.selection.is_collapsed() {
            return Err(EditorError::NothingToDo);
        }

        if let Some((id, start, end)) = self.selection.single_block_range() {
            let block = self
                .document
                .block(id)
                .ok_or(EditorError::NoSuchBlock(id))?;

            if let Some(code) = block.kind.code() {
                let characters: Vec<char> = code.chars().collect();
                let (start, end) = (start.min(characters.len()), end.min(characters.len()));
                // The whole of a code block keeps its fence and its language;
                // part of one is text, and a fence around half a line would be
                // a lie about what was selected.
                if start == 0 && end == characters.len() {
                    return Ok(Fragment::Blocks {
                        blocks: vec![block.clone()],
                    });
                }
                return Ok(Fragment::Text {
                    text: characters[start..end].iter().collect(),
                });
            }

            let content = block.content().ok_or(EditorError::NotTextual(id))?;
            if start == 0 && end >= content.len() {
                return Ok(Fragment::Blocks {
                    blocks: vec![block.clone()],
                });
            }
            return Ok(Fragment::Inline {
                content: content.slice(start, end),
            });
        }

        let (from, to) = self.selection.ordered(&self.document);
        self.guard_span(from.block, to.block)?;
        let first_top = self
            .document
            .top_ancestor(from.block)
            .and_then(|id| self.document.top_index(id))
            .ok_or(EditorError::NoSuchBlock(from.block))?;
        let last_top = self
            .document
            .top_ancestor(to.block)
            .and_then(|id| self.document.top_index(id))
            .ok_or(EditorError::NoSuchBlock(to.block))?;

        let run = self.document.blocks()[first_top..=last_top].to_vec();
        let mut phase = Phase::Before;
        let mut blocks = keep_covered(run, from.block, to.block, &mut phase);
        // The ends are trimmed after the walk rather than during it: the walk
        // works on ids, and until it is done there is no telling which copy of
        // a block is the one that survived.
        trim_start(&mut blocks, from.block, from.offset);
        trim_end(&mut blocks, to.block, to.offset);
        Ok(Fragment::Blocks { blocks })
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
            // A paste may need two transactions, so it commits its own rather
            // than handing one back to `build`.
            EditorCommand::InsertMarkdown(source) => self.insert_markdown(source, created_at),
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
            EditorCommand::SetSelection(_)
            | EditorCommand::Undo
            | EditorCommand::Redo
            | EditorCommand::InsertMarkdown(_) => {
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
        let coalescing = label == TYPING;
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

    /// Parses pasted Markdown and puts it in, replacing the selection.
    fn insert_markdown(&mut self, source: String, created_at: Option<i64>) -> Result<Change> {
        if source.is_empty() {
            return Err(EditorError::NothingToDo);
        }

        // Inside a code block the text goes in as it is. Parsing it would turn
        // a pasted fence into a block, and a fence inside a fence is not what
        // anyone means by pasting code into code.
        if self.caret_holds_code() && self.selection.single_block_range().is_some() {
            let transaction = self.insert_code(source)?;
            return self.commit(transaction, PASTE, created_at);
        }

        let mut ids = IdGenerator::default();
        let parsed = markdown::parse_blocks(&source, &mut ids);
        let blocks = self.document.adopt(parsed);

        // Markdown that parses to no blocks is still characters the writer
        // asked for — whitespace on its own, or syntax carrying no content —
        // so it goes in as text rather than being dropped.
        if blocks.is_empty() {
            let transaction = self.insert_content(InlineContent::empty(), source)?;
            return self.commit(transaction, PASTE, created_at);
        }

        // One paragraph is a run of characters, not a block: pasting a
        // sentence into the middle of a heading leaves one heading, and the
        // marks the Markdown carried come with it.
        if let [block] = blocks.as_slice() {
            if let BlockKind::Paragraph { content } = &block.kind {
                let content = restore_edge_spaces(content.clone(), &source);
                let transaction = self.insert_content(content, String::new())?;
                return self.commit(transaction, PASTE, created_at);
            }
        }

        // Whole blocks. A selection is deleted first, in a transaction of its
        // own: the splice below is built from the block list the delete
        // leaves behind, so it cannot be built before the delete has run.
        let mut change = Change::default();
        let deleted = if self.selection.is_collapsed() {
            false
        } else {
            let transaction = self.delete(Direction::Backward)?;
            change.merge(self.commit(transaction, PASTE, created_at)?);
            true
        };

        let transaction = self.splice_at_caret(blocks)?;
        change.merge(if deleted {
            self.commit_folded(transaction)?
        } else {
            self.commit(transaction, PASTE, created_at)?
        });
        Ok(change)
    }

    /// Puts whole blocks in at a collapsed caret, splitting the block the
    /// caret sits in around them.
    ///
    /// The text before the caret keeps the block's id and kind, and the first
    /// pasted paragraph joins onto it — so pasting two paragraphs into the
    /// middle of a sentence gives three blocks, not four, and the half
    /// sentence before the caret isn't left in a block of its own. The text
    /// after the caret joins the last pasted paragraph the same way.
    fn splice_at_caret(&mut self, mut blocks: Vec<Block>) -> Result<Transaction> {
        let caret_block = self.selection.head.block;
        let site = self.site(caret_block)?;
        let offset = self.selection.head.offset;
        let existing = self
            .document
            .block(caret_block)
            .ok_or(EditorError::NoSuchBlock(caret_block))?;

        // A block with no runs to split — a rule, an image, a code block the
        // selection ran out of — takes the paste after it, whole.
        let Some(content) = existing.content() else {
            let caret = end_of_last_text_block(&blocks).ok_or(EditorError::Unsupported(
                "pasting blocks with no text in them",
            ))?;
            let insert = self.splice(site, site.index + 1, site.index + 1, blocks)?;
            return Ok(
                Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(insert),
            );
        };

        let heading_level = match &existing.kind {
            BlockKind::Heading { level, .. } => Some(*level),
            _ => None,
        };
        let head = content.slice(0, offset);
        let tail = content.slice(offset, content.len());

        // The first pasted paragraph is text, so it joins the text before the
        // caret instead of becoming a block of its own.
        let mut lead = head;
        if matches!(
            blocks.first().map(|b| &b.kind),
            Some(BlockKind::Paragraph { .. })
        ) {
            let first = blocks.remove(0);
            if let BlockKind::Paragraph { content } = first.kind {
                lead = join_content(&lead, &content);
            }
        }
        // Nothing before the caret means the block contributes nothing: the
        // text after the caret has already gone onto the end of the paste, so
        // keeping it would leave an empty paragraph above what was pasted.
        // This is what makes pasting into an empty block replace it.
        let lead_length = lead.len();
        let first = (lead_length > 0).then(|| {
            Block::new(
                caret_block,
                match heading_level {
                    Some(level) => BlockKind::Heading {
                        level,
                        content: lead,
                    },
                    None => BlockKind::Paragraph { content: lead },
                },
            )
        });

        // Where the caret lands: the end of what was pasted, which is the end
        // of the last pasted block before the tail is appended to it. A paste
        // with no text in it at all — a rule, a divider — leaves the caret
        // where the lead ends.
        let caret = end_of_last_text_block(&blocks)
            .or_else(|| {
                first
                    .as_ref()
                    .map(|block| Position::new(block.id, lead_length))
            })
            .unwrap_or(self.selection.head);

        if !tail.is_empty() {
            match blocks.last_mut().map(|block| &mut block.kind) {
                // The text after the caret joins the last pasted paragraph.
                Some(BlockKind::Paragraph { content }) => {
                    *content = join_content(content, &tail);
                }
                // Nothing to join it to — the paste ended in a list, a table,
                // a fence — so it carries on in a paragraph of its own.
                _ => {
                    let block = self
                        .document
                        .new_block(BlockKind::Paragraph { content: tail });
                    blocks.push(block);
                }
            }
        }

        let mut list = Vec::with_capacity(blocks.len() + 1);
        list.extend(first);
        list.extend(blocks);
        let splice = self.splice(site, site.index, site.index + 1, list)?;
        Ok(Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(splice))
    }

    /// Applies a transaction and folds it into the history entry the previous
    /// one made, so a command that takes two transactions is one press of
    /// undo. Records normally when there is no entry to fold into.
    fn commit_folded(&mut self, transaction: Transaction) -> Result<Change> {
        let inverse = transaction.apply(&mut self.document)?;
        let change = transaction.change();
        self.selection = transaction
            .selection_after
            .clamp(&self.document, start_of_document(&self.document));
        self.pending_marks = None;
        if !self.history.fold(inverse.clone(), transaction.clone()) {
            self.history
                .record(inverse, transaction, Some(PASTE.to_string()), None);
        }
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
    /// everything the selection covers in between is removed, including quotes
    /// and lists left with nothing in them. The ends may sit at different
    /// depths; only a selection crossing a table's edge is refused, since
    /// removing cells would leave rows of different lengths.
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
        self.join_ops(from, to, content)
    }

    /// The operations that replace everything between `from` and `to` with
    /// `content`, joining the two blocks into one.
    ///
    /// Both a selection being typed over and a backspace at the start of a
    /// block come through here, so the seam between two blocks behaves the same
    /// way however it is closed.
    fn join_ops(
        &self,
        from: Position,
        to: Position,
        content: InlineContent,
    ) -> Result<(Vec<Operation>, Position)> {
        self.guard_join(from.block, to.block)?;

        // Code is kept rather than joined: prose can't absorb it without
        // losing its fence, so the code after the selection stays where it is.
        let end_holds_code = self
            .document
            .block(to.block)
            .map(|block| block.kind.code().is_some())
            .unwrap_or(false);
        let end_code_tail = end_holds_code && to.offset < self.document.block_len(to.block);

        // What survives in the last block joins the first one, right after the
        // inserted content.
        let tail = if end_holds_code {
            InlineContent::default()
        } else {
            self.document
                .block(to.block)
                .and_then(Block::content)
                .map(|content| content.slice(to.offset, content.len()))
                .unwrap_or_default()
        };
        let caret = Position::new(from.block, from.offset + content.len());
        let mut joined: Vec<Inline> = content.pieces().to_vec();
        joined.extend(tail.pieces().iter().cloned());

        // The structural operation comes first: it is built from the document
        // as it stands, so an inline edit applied before it would be lost when
        // the blocks it rewrites are put back.
        let mut operations =
            vec![self.prune_selection_op(from.block, to.block, end_code_tail, None)?];
        operations.push(Operation::ReplaceInline {
            block: from.block,
            start: from.offset,
            end: self.document.block_len(from.block),
            content: InlineContent::new(joined),
        });
        if end_code_tail {
            // The selected head of the code block goes; the rest stays.
            operations.push(Operation::ReplaceCode {
                block: to.block,
                start: 0,
                end: to.offset,
                text: String::new(),
            });
        }
        Ok((operations, caret))
    }

    /// What both ends of any cross-block operation have to satisfy: in
    /// document order, and on the same side of every table's edge — neither a
    /// prune nor a copy can keep a table's rows the same length across one.
    fn guard_span(&self, from: BlockId, to: BlockId) -> Result<()> {
        let first_order = self
            .document
            .order_index(from)
            .ok_or(EditorError::NoSuchBlock(from))?;
        let last_order = self
            .document
            .order_index(to)
            .ok_or(EditorError::NoSuchBlock(to))?;
        if last_order <= first_order {
            return Err(EditorError::Unsupported(
                "a selection whose ends are not in document order",
            ));
        }
        if self.document.table_position(from) != self.document.table_position(to) {
            return Err(EditorError::Unsupported(
                "a selection crossing the edge of a table",
            ));
        }
        Ok(())
    }

    /// The same, for an edit that joins the two ends into one block: the start
    /// block is the one that survives, so it has to hold inline content.
    fn guard_join(&self, from: BlockId, to: BlockId) -> Result<()> {
        self.guard_span(from, to)?;
        self.content_of(from)?;
        Ok(())
    }

    /// The operation removing everything the selection covers between its two
    /// ends, at whatever depth they sit.
    ///
    /// Two blocks in one list are a plain splice of that list. Otherwise the
    /// top-level blocks the selection runs through are rewritten from a pruned
    /// copy: one block when both ends are inside the same one — a quote, or two
    /// items of one list — and the whole run when it spans several.
    ///
    /// `replace_from` puts blocks where the start block was, which is how a
    /// return pressed over a selection splits and deletes in one operation.
    fn prune_selection_op(
        &self,
        from: BlockId,
        to: BlockId,
        keep_end: bool,
        replace_from: Option<Vec<Block>>,
    ) -> Result<Operation> {
        let first = self.site(from)?;
        let last = self.site(to)?;
        if replace_from.is_none() && first.shares_list_with(&last) && !keep_end {
            return self.splice(first, first.index + 1, last.index + 1, Vec::new());
        }

        let first_top = self
            .document
            .top_ancestor(from)
            .and_then(|id| self.document.top_index(id))
            .ok_or(EditorError::NoSuchBlock(from))?;
        let last_top = self
            .document
            .top_ancestor(to)
            .and_then(|id| self.document.top_index(id))
            .ok_or(EditorError::NoSuchBlock(to))?;

        let mut phase = Phase::Before;
        let run = self.document.blocks()[first_top..=last_top].to_vec();
        let mut pruned = prune_covered(run, from, to, keep_end, &mut phase);
        if let Some(blocks) = replace_from {
            if !splice_in_tree(&mut pruned, from, blocks) {
                return Err(EditorError::NoSuchBlock(from));
            }
        }

        // Both ends inside one top-level block: that block is rewritten, which
        // leaves the document's own list untouched and the ids inside it alone.
        if first_top == last_top {
            let block = pruned
                .into_iter()
                .next()
                .ok_or(EditorError::NoSuchBlock(from))?;
            return Ok(Operation::ReplaceBlock {
                block: self.document.blocks()[first_top].id,
                with: Box::new(block),
            });
        }
        Ok(Operation::ReplaceBlocks {
            start: first_top,
            end: last_top + 1,
            blocks: pruned,
        })
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
        if self.caret_holds_code()
            && content.is_empty()
            && self.selection.single_block_range().is_some()
        {
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
        // Code is edited as text, but only when the whole selection is inside
        // it: a selection that runs out of a code block is a block-level edit
        // like any other.
        if self.caret_holds_code() && self.selection.single_block_range().is_some() {
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
        if self.selection.single_block_range().is_none() {
            return self.split_over_selection();
        }
        let (block, start, end, content) = self.inline_target_owned()?;
        let site = self.site(block)?;
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
        let split = self.splice(site, site.index, site.index + 1, vec![first, second])?;
        Ok(Transaction::new(self.selection, SelectionRange::collapsed(caret)).with(split))
    }

    /// Return pressed with a selection that spans blocks: the selection goes
    /// and the split happens where it was, in one operation rather than a
    /// delete the writer would have to undo twice.
    fn split_over_selection(&mut self) -> Result<Transaction> {
        let (from, to) = self.selection.ordered(&self.document);
        self.guard_join(from.block, to.block)?;
        let content = self.content_of(from.block)?.clone();

        let (tail, keep_end) = match self.document.block(to.block) {
            Some(block) if block.kind.code().is_some() => (
                InlineContent::default(),
                to.offset < self.document.block_len(to.block),
            ),
            Some(block) => (
                block
                    .content()
                    .map(|content| content.slice(to.offset, content.len()))
                    .unwrap_or_default(),
                false,
            ),
            None => return Err(EditorError::NoSuchBlock(to.block)),
        };

        let head = content.slice(0, from.offset);
        let heading_level = match &self.document.block(from.block).expect("checked above").kind {
            BlockKind::Heading { level, .. } => Some(*level),
            _ => None,
        };
        let first = Block::new(
            from.block,
            match heading_level {
                Some(level) => BlockKind::Heading {
                    level,
                    content: head,
                },
                None => BlockKind::Paragraph { content: head },
            },
        );
        let second = self
            .document
            .new_block(BlockKind::Paragraph { content: tail });
        let caret = Position::new(second.id, 0);

        let mut transaction = Transaction::new(self.selection, SelectionRange::collapsed(caret));
        transaction.operations.push(self.prune_selection_op(
            from.block,
            to.block,
            keep_end,
            Some(vec![first, second]),
        )?);
        if keep_end {
            transaction.operations.push(Operation::ReplaceCode {
                block: to.block,
                start: 0,
                end: to.offset,
                text: String::new(),
            });
        }
        Ok(transaction)
    }

    fn merge_blocks(&mut self) -> Result<Transaction> {
        let (block, _, _, _) = self.inline_target()?;
        self.merge(block)
    }

    /// Joins `block` onto the text block before it, wherever the two sit.
    ///
    /// This is the same edit as deleting a selection that runs from the end of
    /// the previous block to the start of this one, and it is built that way:
    /// one seam, one behaviour. A quote or a list left empty by the join is
    /// removed with it.
    fn merge(&self, block: BlockId) -> Result<Transaction> {
        let previous = self
            .document
            .text_block_before(block)
            .ok_or(EditorError::NothingToDo)?;
        // Code can't absorb prose or be absorbed by it without losing its
        // fence, so a backspace at that seam does nothing rather than failing
        // halfway through a transaction.
        let joins_code = [block, previous].iter().any(|id| {
            self.document
                .block(*id)
                .map(|block| block.kind.code().is_some())
                .unwrap_or(false)
        });
        if joins_code {
            return Err(EditorError::NothingToDo);
        }
        self.content_of(block)?;

        let joint = self.document.block_len(previous);
        let (operations, caret) = self.join_ops(
            Position::new(previous, joint),
            Position::new(block, 0),
            InlineContent::empty(),
        )?;
        let mut transaction = Transaction::new(self.selection, SelectionRange::collapsed(caret));
        transaction.operations = operations;
        Ok(transaction)
    }

    /// The text blocks a selection spanning blocks covers, in document order,
    /// at whatever depth they sit.
    fn covered_text_blocks(&self) -> Result<Vec<BlockId>> {
        let (from, to) = self.selection.ordered(&self.document);
        let ids = self.document.text_block_ids();
        let first = ids
            .iter()
            .position(|id| *id == from.block)
            .ok_or(EditorError::NoSuchBlock(from.block))?;
        let last = ids
            .iter()
            .position(|id| *id == to.block)
            .ok_or(EditorError::NoSuchBlock(to.block))?;
        if last < first {
            return Err(EditorError::Unsupported(
                "a selection whose ends are not in document order",
            ));
        }
        Ok(ids[first..=last].to_vec())
    }

    /// The run of sibling blocks a selection covers, as the list it sits in and
    /// the first and last index in it.
    ///
    /// A command that puts one container around what is selected needs the
    /// selection to be one run of siblings; ends in different lists — two list
    /// items, two table cells, inside and outside a quote — are refused, since
    /// there is no single place for the container to go.
    fn covered_run(&self) -> Result<(Site, usize)> {
        let (from, to) = self.selection.ordered(&self.document);
        let first = self.site(from.block)?;
        let last = self.site(to.block)?;
        if !first.shares_list_with(&last) {
            return Err(EditorError::Unsupported(
                "a selection whose ends are in different containers",
            ));
        }
        if last.index < first.index {
            return Err(EditorError::Unsupported(
                "a selection whose ends are not in document order",
            ));
        }
        Ok((first, last.index))
    }

    fn toggle_mark(&self, mark: MarkSet) -> Result<Transaction> {
        if self.selection.single_block_range().is_none() {
            return self.toggle_mark_across_blocks(mark);
        }
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

    /// Toggles a mark over a selection that spans blocks, one inline edit per
    /// block it covers.
    ///
    /// Marks change no structure, so this works at any depth and across a quote
    /// or a list. It is on unless every covered run already carries it — the
    /// same rule as inside one block — and a code block in the way is left
    /// alone, since code carries no marks.
    fn toggle_mark_across_blocks(&self, mark: MarkSet) -> Result<Transaction> {
        let (from, to) = self.selection.ordered(&self.document);
        let ids = self.document.text_block_ids();
        let first = ids
            .iter()
            .position(|id| *id == from.block)
            .ok_or(EditorError::NoSuchBlock(from.block))?;
        let last = ids
            .iter()
            .position(|id| *id == to.block)
            .ok_or(EditorError::NoSuchBlock(to.block))?;
        if last <= first {
            return Err(EditorError::Unsupported(
                "a selection whose ends are not in document order",
            ));
        }

        let mut spans: Vec<(BlockId, usize, usize)> = Vec::new();
        for (offset, id) in ids[first..=last].iter().enumerate() {
            let Some(content) = self.document.block(*id).and_then(Block::content) else {
                continue;
            };
            let start = if offset == 0 { from.offset } else { 0 };
            let end = if first + offset == last {
                to.offset.min(content.len())
            } else {
                content.len()
            };
            if end > start {
                spans.push((*id, start, end));
            }
        }
        if spans.is_empty() {
            return Err(EditorError::NothingToDo);
        }

        let on = !spans.iter().all(|(id, start, end)| {
            self.document
                .block(*id)
                .and_then(Block::content)
                .map(|content| content.has_mark_throughout(*start, *end, mark))
                .unwrap_or(false)
        });

        let mut transaction = Transaction::new(self.selection, self.selection);
        for (block, start, end) in spans {
            let content = self.content_of(block)?;
            let mut middle = content.slice(start, end);
            if on {
                middle.apply_marks(0, middle.len(), mark, MarkSet::NONE);
            } else {
                middle.apply_marks(0, middle.len(), MarkSet::NONE, mark);
            }
            transaction = transaction.with(Operation::ReplaceInline {
                block,
                start,
                end,
                content: middle,
            });
        }
        Ok(transaction)
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
        if self.selection.single_block_range().is_none() {
            return self.set_heading_level_across_blocks(level);
        }
        let block = self.selection.head.block;
        let existing = self
            .document
            .block(block)
            .ok_or(EditorError::NoSuchBlock(block))?;

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
            Transaction::new(self.selection, self.selection).with(Operation::ReplaceBlock {
                block,
                with: Box::new(Block::new(block, kind)),
            }),
        )
    }

    /// Sets the heading level on every block a selection spanning blocks
    /// covers. One replacement each, so a block keeps its id and its place.
    /// Code blocks in the way are left as code.
    fn set_heading_level_across_blocks(&self, level: u8) -> Result<Transaction> {
        let mut transaction = Transaction::new(self.selection, self.selection);
        for id in self.covered_text_blocks()? {
            let Some(content) = self.document.block(id).and_then(Block::content) else {
                continue;
            };
            let kind = if level == 0 {
                BlockKind::Paragraph {
                    content: content.clone(),
                }
            } else {
                BlockKind::heading(level, content.clone())
            };
            transaction = transaction.with(Operation::ReplaceBlock {
                block: id,
                with: Box::new(Block::new(id, kind)),
            });
        }
        if transaction.is_empty() {
            return Err(EditorError::NothingToDo);
        }
        Ok(transaction)
    }

    fn set_code_block(&self, language: Option<String>) -> Result<Transaction> {
        let block = self.selection.head.block;
        self.reject_container_in_cell(block, "a code block inside a table cell")?;
        let existing = self
            .document
            .block(block)
            .ok_or(EditorError::NoSuchBlock(block))?;
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
                Operation::ReplaceBlock {
                    block,
                    with: Box::new(Block::new(block, BlockKind::CodeBlock { language, code })),
                },
            ),
        )
    }

    fn wrap_in_quote(&mut self) -> Result<Transaction> {
        if self.selection.single_block_range().is_none() {
            return self.wrap_run_in_quote();
        }
        let block = self.selection.head.block;
        self.reject_container_in_cell(block, "a quote inside a table cell")?;
        let inner = self
            .document
            .block(block)
            .ok_or(EditorError::NoSuchBlock(block))?
            .clone();
        // The quoted block keeps its id, so the caret inside it stays valid.
        let quote = self.document.new_block(BlockKind::Quote {
            blocks: vec![inner],
        });

        Ok(
            Transaction::new(self.selection, self.selection).with(Operation::ReplaceBlock {
                block,
                with: Box::new(quote),
            }),
        )
    }

    /// Puts one quote around every block a selection spanning blocks covers.
    fn wrap_run_in_quote(&mut self) -> Result<Transaction> {
        let (run, last) = self.covered_run()?;
        self.reject_container_in_cell(run.anchor, "a quote inside a table cell")?;
        let inner = self.sibling_run(run, last)?;
        let quote = self.document.new_block(BlockKind::Quote { blocks: inner });
        let wrap = self.splice(run, run.index, last + 1, vec![quote])?;
        Ok(Transaction::new(self.selection, self.selection).with(wrap))
    }

    /// The blocks of `site`'s list from its index through `last`.
    fn sibling_run(&self, site: Site, last: usize) -> Result<Vec<Block>> {
        let list: &[Block] = match site.parent {
            None => self.document.blocks(),
            Some(parent) => self
                .document
                .block(parent)
                .and_then(|block| block.child_list(site.anchor))
                .ok_or(EditorError::NoSuchBlock(parent))?,
        };
        if last >= list.len() {
            return Err(EditorError::OutOfRange {
                start: site.index,
                end: last + 1,
                len: list.len(),
            });
        }
        Ok(list[site.index..=last].to_vec())
    }

    fn insert_block_after_caret(
        &mut self,
        kind: BlockKind,
        move_caret: bool,
    ) -> Result<Transaction> {
        let block = self.selection.head.block;
        let site = self.site(block)?;
        let inserted = self.document.new_block(kind);
        let id = inserted.id;

        let after = if move_caret {
            SelectionRange::collapsed(Position::new(id, 0))
        } else {
            self.selection
        };
        let insert = self.splice(site, site.index + 1, site.index + 1, vec![inserted])?;
        Ok(Transaction::new(self.selection, after).with(insert))
    }

    fn toggle_list(&mut self, ordered: bool) -> Result<Transaction> {
        if self.selection.single_block_range().is_none() {
            return self.list_run(ordered);
        }
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
                let site = self.site(list_id)?;
                let blocks: Vec<Block> = items.into_iter().flat_map(|item| item.blocks).collect();
                // A nested list unwraps into the item or quote around it, which
                // is one level of indentation gone rather than the lot.
                let unwrap = self.splice(site, site.index, site.index + 1, blocks)?;
                return Ok(Transaction::new(self.selection, self.selection).with(unwrap));
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

        self.reject_container_in_cell(block, "a list inside a table cell")?;
        let site = self.site(block)?;
        let inner = self
            .document
            .block(block)
            .ok_or(EditorError::NoSuchBlock(block))?
            .clone();
        let list = self.document.new_block(BlockKind::List {
            ordered,
            start: None,
            tight: true,
            items: vec![ListItem::new(vec![inner])],
        });
        let wrap = self.splice(site, site.index, site.index + 1, vec![list])?;
        Ok(Transaction::new(self.selection, self.selection).with(wrap))
    }

    /// Makes one list of every block a selection spanning blocks covers, one
    /// item per block — the usual way a few lines become a list.
    fn list_run(&mut self, ordered: bool) -> Result<Transaction> {
        let (run, last) = self.covered_run()?;
        self.reject_container_in_cell(run.anchor, "a list inside a table cell")?;
        let items: Vec<ListItem> = self
            .sibling_run(run, last)?
            .into_iter()
            .map(|block| ListItem::new(vec![block]))
            .collect();
        let list = self.document.new_block(BlockKind::List {
            ordered,
            start: None,
            tight: true,
            items,
        });
        let wrap = self.splice(run, run.index, last + 1, vec![list])?;
        Ok(Transaction::new(self.selection, self.selection).with(wrap))
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
        self.reject_container_in_cell(block, "a table inside a table cell")?;
        let site = self.site(block)?;
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

        let insert = self.splice(site, site.index + 1, site.index + 1, vec![table])?;
        Ok(Transaction::new(
            self.selection,
            SelectionRange::collapsed(Position::new(caret, 0)),
        )
        .with(insert))
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

    /// Refuses a container — a quote, list, code block or table — where
    /// Markdown cannot write one. A table cell holds inline content only, so a
    /// container put in one would be dropped the next time the document is
    /// written out.
    fn reject_container_in_cell(&self, block: BlockId, what: &'static str) -> Result<()> {
        if self.document.table_position(block).is_some() {
            return Err(EditorError::Unsupported(what));
        }
        Ok(())
    }

    /// Where `block` sits among its siblings.
    fn site(&self, block: BlockId) -> Result<Site> {
        let (parent, list, index) = self
            .document
            .sibling_position(block)
            .ok_or(EditorError::NoSuchBlock(block))?;
        Ok(Site {
            parent,
            list,
            anchor: block,
            index,
        })
    }

    /// The operation replacing `start..end` of the sibling list `site` sits in.
    ///
    /// At top level that is a block-list splice. Inside a quote, a list item or
    /// a table cell the container is rewritten instead, with every block that
    /// survives keeping its id so carets and annotations pointing into it stay
    /// valid.
    fn splice(
        &self,
        site: Site,
        start: usize,
        end: usize,
        blocks: Vec<Block>,
    ) -> Result<Operation> {
        let Some(parent) = site.parent else {
            return Ok(Operation::ReplaceBlocks { start, end, blocks });
        };
        let mut container = self
            .document
            .block(parent)
            .ok_or(EditorError::NoSuchBlock(parent))?
            .clone();
        let list = container
            .child_list_mut(site.anchor)
            .ok_or(EditorError::NoSuchBlock(site.anchor))?;
        if start > end || end > list.len() {
            return Err(EditorError::OutOfRange {
                start,
                end,
                len: list.len(),
            });
        }
        list.splice(start..end, blocks);
        Ok(Operation::ReplaceBlock {
            block: parent,
            with: Box::new(container),
        })
    }
}

/// Where a block sits among its siblings, for the structural edits that splice
/// a list of blocks: the container holding it (`None` at top level), the list
/// itself, named by its first block, and the block's index in that list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Site {
    parent: Option<BlockId>,
    list: BlockId,
    anchor: BlockId,
    index: usize,
}

impl Site {
    /// Whether both blocks sit in the same list, so one edit can span them.
    /// Separate list items and separate table cells share a parent without
    /// sharing a list.
    fn shares_list_with(&self, other: &Site) -> bool {
        self.parent == other.parent && self.list == other.list
    }
}

/// How far a walk over the block tree has got through a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Before the block the selection starts in.
    Before,
    /// Between the selection's two ends: what is here is covered by it.
    Inside,
    /// Past the block the selection ends in.
    After,
}

/// Puts back the spaces and tabs Markdown drops from the edges of a block.
///
/// `  and more` is a paragraph of `and more` to a Markdown parser: whitespace
/// at the edge of a block carries no meaning there. Pasted into the middle of
/// a sentence it carries all of it, so it is restored — but only for a paste
/// that was one line to begin with, since across a block break the whitespace
/// really is insignificant.
fn restore_edge_spaces(content: InlineContent, source: &str) -> InlineContent {
    let space = |c: char| c == ' ' || c == '\t';
    if source.contains('\n') {
        return content;
    }
    let leading: String = source.chars().take_while(|c| space(*c)).collect();
    let trailing: String = source
        .chars()
        .rev()
        .take_while(|c| space(*c))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if leading.is_empty() && trailing.is_empty() {
        return content;
    }
    let mut pieces = Vec::with_capacity(content.pieces().len() + 2);
    if !leading.is_empty() {
        pieces.push(Inline::text(leading));
    }
    pieces.extend(content.pieces().iter().cloned());
    if !trailing.is_empty() {
        pieces.push(Inline::text(trailing));
    }
    InlineContent::new(pieces)
}

/// Two runs of inline content, one after the other.
fn join_content(first: &InlineContent, second: &InlineContent) -> InlineContent {
    let mut pieces = first.pieces().to_vec();
    pieces.extend(second.pieces().iter().cloned());
    InlineContent::new(pieces)
}

/// The end of the last block in `blocks` a caret can go in, at any depth —
/// where a paste leaves the caret. `None` when nothing pasted holds text.
fn end_of_last_text_block(blocks: &[Block]) -> Option<Position> {
    blocks
        .iter()
        .flat_map(Block::descendants)
        .rfind(|block| block.kind.holds_text())
        .map(|block| {
            let len = match (block.content(), block.kind.code()) {
                (Some(content), _) => content.len(),
                (None, Some(code)) => code.chars().count(),
                _ => 0,
            };
            Position::new(block.id, len)
        })
}

/// Keeps only what a selection covers, the mirror of [`prune_covered`].
///
/// Containers are kept when the selection runs through them, holding only the
/// children it reached, so copying from the middle of one list item to the
/// middle of another gives back a list of two items rather than two loose
/// paragraphs. Both end blocks are kept whole here and trimmed to their
/// offsets afterwards.
fn keep_covered(list: Vec<Block>, from: BlockId, to: BlockId, phase: &mut Phase) -> Vec<Block> {
    let mut out = Vec::new();
    for block in list {
        match phase {
            Phase::Before => {
                if block.id == from {
                    *phase = Phase::Inside;
                    out.push(block);
                } else if block.find(from).is_some() {
                    let kept = keep_children(block, from, to, phase);
                    if !kept.children().is_empty() {
                        out.push(kept);
                    }
                }
                // Anything wholly before the start is not part of the copy.
            }
            Phase::Inside => {
                if block.id == to {
                    *phase = Phase::After;
                    out.push(block);
                } else if block.find(to).is_some() {
                    let kept = keep_children(block, from, to, phase);
                    if !kept.children().is_empty() {
                        out.push(kept);
                    }
                } else {
                    out.push(block);
                }
            }
            Phase::After => {}
        }
    }
    out
}

/// The same walk, through one block's children. Tables are left whole: a
/// selection crossing a table's edge is refused by [`Editor::guard_span`], so
/// a table reached here is one the selection covers entirely.
fn keep_children(mut block: Block, from: BlockId, to: BlockId, phase: &mut Phase) -> Block {
    match &mut block.kind {
        BlockKind::Quote { blocks } => {
            let taken = std::mem::take(blocks);
            *blocks = keep_covered(taken, from, to, phase);
        }
        BlockKind::List { items, .. } => {
            let mut kept = Vec::new();
            for mut item in std::mem::take(items) {
                let taken = std::mem::take(&mut item.blocks);
                item.blocks = keep_covered(taken, from, to, phase);
                // An item the selection never reached is not an empty bullet.
                if !item.blocks.is_empty() {
                    kept.push(item);
                }
            }
            *items = kept;
        }
        _ => {}
    }
    block
}

/// Drops what comes before `offset` in the copied tree's first text block.
fn trim_start(blocks: &mut [Block], id: BlockId, offset: usize) {
    trim(blocks, id, |len| (offset.min(len), len));
}

/// Drops what comes after `offset` in the copied tree's last text block.
fn trim_end(blocks: &mut [Block], id: BlockId, offset: usize) {
    trim(blocks, id, |len| (0, offset.min(len)));
}

/// Cuts a block's text down to the range `bounds` gives for its length.
/// A block holding neither runs nor code — a rule, an image — has no offsets
/// to trim to and is left as it is.
fn trim(blocks: &mut [Block], id: BlockId, bounds: impl Fn(usize) -> (usize, usize)) {
    let Some(block) = blocks.iter_mut().find_map(|block| block.find_mut(id)) else {
        return;
    };
    if let Some(content) = block.content_mut() {
        let (start, end) = bounds(content.len());
        *content = content.slice(start, end);
        return;
    }
    if let BlockKind::CodeBlock { code, .. } = &mut block.kind {
        let characters: Vec<char> = code.chars().collect();
        let (start, end) = bounds(characters.len());
        *code = characters[start..end].iter().collect();
    }
}

/// Removes what a selection covers from a list of blocks, containers included.
///
/// The block the selection starts in stays, with its id: joining the two ends
/// is an inline edit on it, applied after this. The block it ends in goes,
/// unless `keep_end` — a code block keeps the code that follows the selection,
/// since there is no joining prose and code. Containers left holding nothing
/// are removed with their contents, which is what makes deleting a whole quote
/// or list from the middle of one block to the middle of another work.
fn prune_covered(
    list: Vec<Block>,
    from: BlockId,
    to: BlockId,
    keep_end: bool,
    phase: &mut Phase,
) -> Vec<Block> {
    let mut out = Vec::new();
    for block in list {
        match phase {
            Phase::Before => {
                if block.id == from {
                    *phase = Phase::Inside;
                    out.push(block);
                } else if block.find(from).is_some() {
                    // The container holding the start always survives: the
                    // start block is still in it.
                    out.push(prune_children(block, from, to, keep_end, phase));
                } else {
                    out.push(block);
                }
            }
            Phase::Inside => {
                if block.id == to {
                    *phase = Phase::After;
                    if keep_end {
                        out.push(block);
                    }
                } else if block.find(to).is_some() {
                    let pruned = prune_children(block, from, to, keep_end, phase);
                    // Emptied by the deletion: the container goes too.
                    if !pruned.children().is_empty() {
                        out.push(pruned);
                    }
                }
                // Anything else here is covered by the selection, so it goes.
            }
            Phase::After => out.push(block),
        }
    }
    out
}

/// The same walk, through one block's children. Tables are left alone: a
/// selection crossing a table's edge is refused rather than pruned, because
/// removing cells would leave rows of different lengths.
fn prune_children(
    mut block: Block,
    from: BlockId,
    to: BlockId,
    keep_end: bool,
    phase: &mut Phase,
) -> Block {
    match &mut block.kind {
        BlockKind::Quote { blocks } => {
            let taken = std::mem::take(blocks);
            *blocks = prune_covered(taken, from, to, keep_end, phase);
        }
        BlockKind::List { items, .. } => {
            let mut kept = Vec::new();
            for mut item in std::mem::take(items) {
                let taken = std::mem::take(&mut item.blocks);
                item.blocks = prune_covered(taken, from, to, keep_end, phase);
                // An item with nothing left in it is not an empty bullet, it
                // is an item the selection covered.
                if !item.blocks.is_empty() {
                    kept.push(item);
                }
            }
            *items = kept;
        }
        _ => {}
    }
    block
}

/// Puts `blocks` where `target` sits in a tree of blocks, at any depth.
///
/// Used on a tree that has already been pruned, so the replacement lands in the
/// document that the deletion leaves behind rather than the one before it.
fn splice_in_tree(list: &mut Vec<Block>, target: BlockId, blocks: Vec<Block>) -> bool {
    if let Some(index) = list.iter().position(|block| block.id == target) {
        list.splice(index..index + 1, blocks);
        return true;
    }
    for block in list.iter_mut() {
        let lists: Vec<&mut Vec<Block>> = match &mut block.kind {
            BlockKind::Quote { blocks } => vec![blocks],
            BlockKind::List { items, .. } => {
                items.iter_mut().map(|item| &mut item.blocks).collect()
            }
            BlockKind::Table { table } => table
                .rows
                .iter_mut()
                .flat_map(|row| row.cells.iter_mut())
                .map(|cell| &mut cell.blocks)
                .collect(),
            _ => Vec::new(),
        };
        for nested in lists {
            if splice_in_tree(nested, target, blocks.clone()) {
                return true;
            }
        }
    }
    false
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
