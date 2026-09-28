//! Stable identifiers.
//!
//! Blocks are addressed by id, never by position. A position that names a
//! block index goes stale as soon as anything above it is inserted or
//! removed; an id survives every edit that doesn't delete the block itself,
//! which is what selections, comments, grammar results and (later)
//! collaborative cursors need.

use serde::{Deserialize, Serialize};

/// Identifies one document within a session. The engine never interprets it;
/// the host maps it to whatever it uses (for Memorie, a writing's path).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub String);

impl DocumentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DocumentId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Identifies a block for the lifetime of a document. Ids are handed out by
/// [`crate::Document`] and are never reused, so a stale id resolves to
/// nothing rather than to the wrong block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BlockId(pub u64);

/// Hands out block ids for one document.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdGenerator {
    next: u64,
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl IdGenerator {
    pub fn next_id(&mut self) -> BlockId {
        let id = BlockId(self.next);
        self.next += 1;
        id
    }

    /// Makes sure ids handed out from now on are above `id`, after loading a
    /// document whose blocks already carry ids.
    pub fn reserve_above(&mut self, id: BlockId) {
        self.next = self.next.max(id.0 + 1);
    }
}
