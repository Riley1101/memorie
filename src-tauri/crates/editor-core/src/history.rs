//! Branching edit history.
//!
//! The shape is deliberately the same as Memorie's `undotree`: an arena of
//! nodes, each with a parent and children, a `current` pointer, and a
//! session-only redo stack; undoing and then editing again branches instead of
//! throwing the old line of work away, and redo follows the newest branch.
//!
//! What differs is what a node stores. `undotree` stores document *text* (a
//! delta against its parent, with periodic full copies), because at file level
//! that is all it has. Here a node stores the two transactions that move
//! between the states, so undo and redo are operations on the document model,
//! carry the selection with them, and cost nothing to build on a save.
//!
//! The two are complementary rather than duplicates, and Phase 6 decides how
//! they meet: the natural split is this history inside a session (keystroke
//! granularity, selection aware) and `undotree` across sessions (saved
//! versions, on disk, branch-preserving). Nothing here writes to disk, so that
//! decision stays open.

use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeIndex(pub usize);

/// One recorded edit: how to undo it, and how to do it again.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub undo: Transaction,
    pub redo: Transaction,
    pub parent: Option<NodeIndex>,
    pub children: Vec<NodeIndex>,
    /// Milliseconds since the epoch, supplied by the host so the engine stays
    /// deterministic under test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    /// What the edit was, for a history UI ("Typing", "Bold", "Paste").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct History {
    nodes: Vec<Entry>,
    current: Option<NodeIndex>,
    /// Cleared whenever an edit branches, so linear redo never jumps lines.
    #[serde(skip, default)]
    redo_stack: Vec<NodeIndex>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nodes(&self) -> &[Entry] {
        &self.nodes
    }

    pub fn current(&self) -> Option<NodeIndex> {
        self.current
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Records an applied edit as a child of the current node and moves to it.
    pub fn record(
        &mut self,
        undo: Transaction,
        redo: Transaction,
        label: Option<String>,
        created_at: Option<i64>,
    ) -> NodeIndex {
        let parent = self.current;
        let index = NodeIndex(self.nodes.len());
        self.nodes.push(Entry {
            undo,
            redo,
            parent,
            children: Vec::new(),
            created_at,
            label,
        });
        if let Some(parent) = parent {
            self.nodes[parent.0].children.push(index);
        }
        self.current = Some(index);
        self.redo_stack.clear();
        index
    }

    /// The transaction that undoes the current edit, and the node to move to.
    /// Returns `None` at the root.
    pub fn undo(&mut self) -> Option<Transaction> {
        let current = self.current?;
        let undo = self.nodes[current.0].undo.clone();
        self.redo_stack.push(current);
        self.current = self.nodes[current.0].parent;
        Some(undo)
    }

    /// The transaction that redoes the edit undone last, or — after branching
    /// away from a linear line — the newest child of the current node.
    pub fn redo(&mut self) -> Option<Transaction> {
        let target = match self.redo_stack.pop() {
            Some(index) => index,
            None => *self.children_newest_first(self.current).first()?,
        };
        let redo = self.nodes[target.0].redo.clone();
        self.current = Some(target);
        Some(redo)
    }

    /// Children of a node (or the roots, for `None`), newest first. Nodes are
    /// only ever appended, so a higher index means a later edit.
    pub fn children_newest_first(&self, node: Option<NodeIndex>) -> Vec<NodeIndex> {
        let mut children: Vec<NodeIndex> = match node {
            Some(node) => self
                .nodes
                .get(node.0)
                .map(|n| n.children.clone())
                .unwrap_or_default(),
            None => (0..self.nodes.len())
                .map(NodeIndex)
                .filter(|index| self.nodes[index.0].parent.is_none())
                .collect(),
        };
        children.sort_by(|a, b| b.cmp(a));
        children
    }
}
