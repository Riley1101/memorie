//! The document: the canonical state the editor works on.
//!
//! Markdown is storage, not state. Nothing in here parses or prints Markdown;
//! that lives in the (Phase 3) `markdown` module, which is the only place that
//! knows the syntax.

use crate::ids::{BlockId, DocumentId, IdGenerator};
use crate::inline::InlineContent;
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
        self.all_blocks().into_iter().find(|block| block.id == id)
    }

    pub fn block_mut(&mut self, id: BlockId) -> Option<&mut Block> {
        self.blocks.iter_mut().find_map(|block| block.find_mut(id))
    }

    /// Position of `id` in the top-level list, if it is a top-level block.
    pub fn top_index(&self, id: BlockId) -> Option<usize> {
        self.blocks.iter().position(|block| block.id == id)
    }

    /// Ids of the blocks that hold editable inline content, in document order.
    /// This is the sequence the caret moves along.
    pub fn text_block_ids(&self) -> Vec<BlockId> {
        self.all_blocks()
            .into_iter()
            .filter(|block| block.kind.is_textual())
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

    /// Offset length of a block's inline content; 0 for blocks without any.
    pub fn block_len(&self, id: BlockId) -> usize {
        self.block(id)
            .and_then(Block::content)
            .map(InlineContent::len)
            .unwrap_or(0)
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
