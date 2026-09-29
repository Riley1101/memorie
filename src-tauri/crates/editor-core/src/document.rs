//! The document: the canonical state the editor works on.
//!
//! Markdown is storage, not state. Nothing in here parses or prints Markdown;
//! that lives in the (Phase 3) `markdown` module, which is the only place that
//! knows the syntax.

use crate::ids::{BlockId, DocumentId, IdGenerator};
use crate::node::{Block, BlockKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Front matter, kept out of the block list so no edit can accidentally turn
/// metadata into a thematic break (a real hazard with `---`).
///
/// `raw` is the block exactly as it was read. A save writes `raw` back
/// unchanged while `fields` are untouched, so opening and saving a document
/// never reformats somebody's YAML.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frontmatter {
    pub raw: String,
    /// Scalar fields the host understands (Memorie: `synopsis`, `status`,
    /// `label`). Ordered so serializing is deterministic.
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<Frontmatter>,
}

/// A whole document: metadata plus a flat list of top-level blocks. Nesting
/// (quotes, lists, tables) lives inside those blocks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub metadata: DocumentMetadata,
    blocks: Vec<Block>,
    ids: IdGenerator,
    /// Bumped by every applied transaction, so the frontend can tell whether
    /// the state it holds is current without diffing it.
    revision: u64,
}

impl Document {
    /// An empty document: one empty paragraph, so there is always somewhere to type.
    pub fn empty(id: impl Into<DocumentId>) -> Self {
        let mut document = Self {
            id: id.into(),
            metadata: DocumentMetadata::default(),
            blocks: Vec::new(),
            ids: IdGenerator::default(),
            revision: 0,
        };
        let paragraph = document.new_block(BlockKind::paragraph(""));
        document.blocks.push(paragraph);
        document
    }

    /// Builds a document from block kinds, handing out ids in order. The
    /// Markdown parser and tests both come in this way.
    pub fn from_kinds(id: impl Into<DocumentId>, kinds: Vec<BlockKind>) -> Self {
        let mut document = Self {
            id: id.into(),
            metadata: DocumentMetadata::default(),
            blocks: Vec::new(),
            ids: IdGenerator::default(),
            revision: 0,
        };
        // Kinds may already contain blocks with ids (a quote holding
        // paragraphs); ids handed out here must not collide with them.
        if let Some(max) = kinds.iter().filter_map(kind_max_id).max() {
            document.ids.reserve_above(max);
        }
        let ids = &mut document.ids;
        let blocks: Vec<Block> = kinds
            .into_iter()
            .map(|kind| document_block(ids, kind))
            .collect();
        document.blocks = blocks;
        if document.blocks.is_empty() {
            let paragraph = document.new_block(BlockKind::paragraph(""));
            document.blocks.push(paragraph);
        }
        document
    }

    /// Builds a document from blocks that already carry ids — what the
    /// Markdown parser produces, since it needs ids for nested blocks too.
    /// Ids handed out afterwards can't collide with them.
    pub fn from_blocks(id: impl Into<DocumentId>, blocks: Vec<Block>) -> Self {
        let mut document = Self {
            id: id.into(),
            metadata: DocumentMetadata::default(),
            blocks,
            ids: IdGenerator::default(),
            revision: 0,
        };
        document.reseal_ids();
        if document.blocks.is_empty() {
            let paragraph = document.new_block(BlockKind::paragraph(""));
            document.blocks.push(paragraph);
        }
        document
    }

    /// Wraps a kind in a block with a fresh id.
    pub fn new_block(&mut self, kind: BlockKind) -> Block {
        document_block(&mut self.ids, kind)
    }

    /// Makes sure ids handed out later can't collide with ones already in the
    /// document, after loading it from a serialized form.
    pub fn reseal_ids(&mut self) {
        let max = self
            .blocks
            .iter()
            .flat_map(Block::descendants)
            .map(|block| block.id)
            .max();
        if let Some(max) = max {
            self.ids.reserve_above(max);
        }
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// The top-level block list, for the structural operations in
    /// [`crate::transaction`]. Not public: every change goes through a
    /// transaction so history and revisions stay honest.
    pub(crate) fn blocks_mut(&mut self) -> &mut Vec<Block> {
        &mut self.blocks
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn bump_revision(&mut self) {
        self.revision += 1;
    }

    /// Every block in document order, nested ones included.
    pub fn all_blocks(&self) -> Vec<&Block> {
        self.blocks.iter().flat_map(Block::descendants).collect()
    }

    pub fn block(&self, id: BlockId) -> Option<&Block> {
        self.blocks.iter().find_map(|block| block.find(id))
    }

    pub fn block_mut(&mut self, id: BlockId) -> Option<&mut Block> {
        self.blocks.iter_mut().find_map(|block| block.find_mut(id))
    }

    /// Position of `id` in the top-level list, if it is a top-level block.
    /// Swaps one block for another, wherever it sits, and returns the old one.
    pub(crate) fn replace_block(&mut self, id: BlockId, with: Block) -> Option<Block> {
        let slot = self.block_mut(id)?;
        Some(std::mem::replace(slot, with))
    }

    /// The block `id` sits directly inside, if it isn't top level.
    pub fn parent_of(&self, id: BlockId) -> Option<BlockId> {
        self.all_blocks()
            .into_iter()
            .find(|block| block.children().iter().any(|child| child.id == id))
            .map(|block| block.id)
    }

    /// The blocks `id` sits inside, innermost first.
    pub fn ancestors(&self, id: BlockId) -> Vec<BlockId> {
        let mut out = Vec::new();
        let mut at = id;
        while let Some(parent) = self.parent_of(at) {
            out.push(parent);
            at = parent;
        }
        out
    }

    /// The innermost list around `id`, with the index of the item holding it.
    pub fn list_position(&self, id: BlockId) -> Option<(BlockId, usize)> {
        let candidates = std::iter::once(id).chain(self.ancestors(id));
        for candidate in candidates {
            let Some(parent) = self.parent_of(candidate).and_then(|p| self.block(p)) else {
                continue;
            };
            if let BlockKind::List { items, .. } = &parent.kind {
                let item = items.iter().position(|item| {
                    item.blocks
                        .iter()
                        .flat_map(Block::descendants)
                        .any(|block| block.id == candidate)
                })?;
                return Some((parent.id, item));
            }
        }
        None
    }

    /// The innermost table around `id`, with the row and column holding it.
    pub fn table_position(&self, id: BlockId) -> Option<(BlockId, usize, usize)> {
        for block in self.all_blocks() {
            let BlockKind::Table { table } = &block.kind else {
                continue;
            };
            for (row_index, row) in table.rows.iter().enumerate() {
                for (column, cell) in row.cells.iter().enumerate() {
                    let holds = cell
                        .blocks
                        .iter()
                        .flat_map(Block::descendants)
                        .any(|inner| inner.id == id);
                    if holds {
                        return Some((block.id, row_index, column));
                    }
                }
            }
        }
        None
    }

    pub fn top_index(&self, id: BlockId) -> Option<usize> {
        self.blocks.iter().position(|block| block.id == id)
    }

    /// Ids of the blocks a caret can go in — inline content or a code block's
    /// text — in document order. This is the sequence the caret moves along.
    pub fn text_block_ids(&self) -> Vec<BlockId> {
        self.all_blocks()
            .into_iter()
            .filter(|block| block.kind.holds_text())
            .map(|block| block.id)
            .collect()
    }

    /// The text block before `id`, for a backspace at the start of a block.
    pub fn text_block_before(&self, id: BlockId) -> Option<BlockId> {
        let ids = self.text_block_ids();
        let at = ids.iter().position(|candidate| *candidate == id)?;
        at.checked_sub(1).map(|before| ids[before])
    }

    /// The text block after `id`, for a delete at the end of a block.
    pub fn text_block_after(&self, id: BlockId) -> Option<BlockId> {
        let ids = self.text_block_ids();
        let at = ids.iter().position(|candidate| *candidate == id)?;
        ids.get(at + 1).copied()
    }

    /// Offset length of a block's text — inline content, or a code block's
    /// code. 0 for a block that holds neither.
    pub fn block_len(&self, id: BlockId) -> usize {
        let Some(block) = self.block(id) else {
            return 0;
        };
        match (block.content(), block.kind.code()) {
            (Some(content), _) => content.len(),
            (None, Some(code)) => code.chars().count(),
            _ => 0,
        }
    }

    /// One character per offset unit in `id`, whether that is inline content or
    /// code. This is what a view reports back after the browser changed it.
    pub fn offset_text(&self, id: BlockId) -> Option<String> {
        let block = self.block(id)?;
        match (block.content(), block.kind.code()) {
            (Some(content), _) => Some(content.offset_text()),
            (None, Some(code)) => Some(code.to_string()),
            _ => None,
        }
    }

    /// The document as plain text — what search, word counts, embeddings and
    /// prompts should see. Blocks are separated by blank lines, as in Markdown,
    /// so line-based tools still line up roughly with the source.
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for block in &self.blocks {
            push_block_text(block, &mut out);
        }
        while out.ends_with('\n') {
            out.pop();
        }
        out
    }

    /// Words in the document, counted without building its text first — this
    /// runs on every keystroke in a host that shows a live count.
    pub fn word_count(&self) -> usize {
        fn count(block: &Block, total: &mut usize) {
            *total += block.own_text().split_whitespace().count();
            for child in block.children() {
                count(child, total);
            }
        }
        let mut total = 0;
        for block in &self.blocks {
            count(block, &mut total);
        }
        total
    }

    /// Headings in document order, for the outline.
    pub fn outline(&self) -> Vec<Heading> {
        self.all_blocks()
            .into_iter()
            .filter_map(|block| match &block.kind {
                BlockKind::Heading { level, content } => Some(Heading {
                    id: block.id,
                    level: *level,
                    text: content.plain_text(),
                }),
                _ => None,
            })
            .collect()
    }
}

/// One entry of the document outline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    pub id: BlockId,
    pub level: u8,
    pub text: String,
}

/// The highest block id already inside a kind's nested blocks, if any.
fn kind_max_id(kind: &BlockKind) -> Option<BlockId> {
    let nested: Vec<&Block> = match kind {
        BlockKind::Quote { blocks } => blocks.iter().collect(),
        BlockKind::List { items, .. } => items.iter().flat_map(|item| item.blocks.iter()).collect(),
        BlockKind::Table { table } => table
            .rows
            .iter()
            .flat_map(|row| row.cells.iter())
            .flat_map(|cell| cell.blocks.iter())
            .collect(),
        _ => Vec::new(),
    };
    nested
        .into_iter()
        .flat_map(Block::descendants)
        .map(|block| block.id)
        .max()
}

fn document_block(ids: &mut IdGenerator, kind: BlockKind) -> Block {
    Block::new(ids.next_id(), kind)
}

fn push_block_text(block: &Block, out: &mut String) {
    let own = block.own_text();
    if !own.is_empty() {
        out.push_str(&own);
        out.push_str("\n\n");
    }
    for child in block.children() {
        push_block_text(child, out);
    }
}
