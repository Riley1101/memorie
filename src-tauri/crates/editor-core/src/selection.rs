//! Selections.
//!
//! A position is a **block id plus an offset inside that block's inline
//! content**, not a block index. Indices shift the moment anything above them
//! is inserted or removed; ids don't. Offsets inside a block still have to be
//! mapped when that block's own text changes, which is what
//! [`SelectionRange::map_after_replace`] does, and it is the only mapping rule
//! the engine needs so far.
//!
//! Anchor and head are kept apart (rather than start/end) so a backwards
//! selection stays backwards — a caret that keeps growing in the direction the
//! writer is dragging.

use crate::document::Document;
use crate::ids::BlockId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub block: BlockId,
    /// Offset in the block's inline content, counted in characters.
    pub offset: usize,
}

impl Position {
    pub fn new(block: BlockId, offset: usize) -> Self {
        Self { block, offset }
    }
}

/// A selection: collapsed when anchor and head are equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRange {
    pub anchor: Position,
    pub head: Position,
}

impl SelectionRange {
    pub fn collapsed(at: Position) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }

    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    /// A selection inside one block, from `start` to `end`.
    pub fn in_block(block: BlockId, start: usize, end: usize) -> Self {
        Self::new(Position::new(block, start), Position::new(block, end))
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    /// Both ends in document order: the earlier one first.
    pub fn ordered(&self, document: &Document) -> (Position, Position) {
        let order = document.text_block_ids();
        let rank = |position: &Position| {
            order
                .iter()
                .position(|id| *id == position.block)
                .unwrap_or(usize::MAX)
        };
        let (a, h) = (self.anchor, self.head);
        match (rank(&a), rank(&h)) {
            (ra, rh) if ra < rh => (a, h),
            (ra, rh) if ra > rh => (h, a),
            _ if a.offset <= h.offset => (a, h),
            _ => (h, a),
        }
    }

    /// Whether both ends sit in the same block, the case every inline
    /// operation in Phase 2 handles.
    pub fn is_within_one_block(&self) -> bool {
        self.anchor.block == self.head.block
    }

    /// The selected range inside a single block, ordered. `None` when the
    /// selection spans blocks.
    pub fn single_block_range(&self) -> Option<(BlockId, usize, usize)> {
        if !self.is_within_one_block() {
            return None;
        }
        let (start, end) = if self.anchor.offset <= self.head.offset {
            (self.anchor.offset, self.head.offset)
        } else {
            (self.head.offset, self.anchor.offset)
        };
        Some((self.anchor.block, start, end))
    }

    /// Moves the selection after `block`'s range `start..end` was replaced
    /// with `new_len` characters.
    ///
    /// A position inside the replaced range collapses to its end, which is
    /// where a caret belongs after the text under it was rewritten; positions
    /// after it shift by the length difference. Positions in other blocks are
    /// untouched, which is the point of addressing blocks by id.
    pub fn map_after_replace(
        self,
        block: BlockId,
        start: usize,
        end: usize,
        new_len: usize,
    ) -> Self {
        let map = |position: Position| {
            if position.block != block {
                return position;
            }
            let offset = if position.offset <= start {
                position.offset
            } else if position.offset >= end {
                position.offset + new_len + start - end
            } else {
                start + new_len
            };
            Position::new(block, offset)
        };
        Self {
            anchor: map(self.anchor),
            head: map(self.head),
        }
    }

    /// Clamps both ends to blocks that still exist and to offsets inside them,
    /// after a structural change. Ends whose block is gone fall back to
    /// `fallback`.
    pub fn clamp(self, document: &Document, fallback: Position) -> Self {
        let fix = |position: Position| match document.block(position.block) {
            Some(_) => Position::new(
                position.block,
                position.offset.min(document.block_len(position.block)),
            ),
            None => fallback,
        };
        Self {
            anchor: fix(self.anchor),
            head: fix(self.head),
        }
    }
}
