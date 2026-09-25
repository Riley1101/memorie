//! Branching version history for every writing.
//!
//! On disk, `undotree_dir` is a directory:
//!
//! ```text
//! history/
//!   index.json     { "<writing name>": "<history id>", ... }
//!   <id>.json      one History per writing
//! ```
//!
//! Each version stores only what changed against its parent (a kept prefix,
//! a kept suffix and the text between), with a full copy every
//! `KEYFRAME_INTERVAL` versions so rebuilding one never walks far. Histories
//! load on first use and a save rewrites only the ones that changed, so a
//! large writing no longer slows down saving every other one.
//!
//! Older versions of the app kept everything, with full copies, in a single
//! JSON file at `undotree_dir`. `UndoTree::load` migrates that file once and
//! keeps it as `<undotree_dir>.v1.json`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// A version is stored in full at least this often along any branch.
const KEYFRAME_INTERVAL: usize = 32;

const INDEX_FILE: &str = "index.json";

/// How one version's text is stored.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeData {
    Full(String),
    /// The parent's text with its middle replaced: its first `keep_start`
    /// bytes, then `insert`, then its last `keep_end` bytes.
    Delta {
        keep_start: usize,
        keep_end: usize,
        insert: String,
    },
}

// Represents a single state in the history tree.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    pub data: NodeData,
    pub parent: Option<NodeIndex>,
    pub children: Vec<NodeIndex>,

    // Milliseconds since the epoch. Histories written before this field
    // existed have no timestamp, so the graph falls back to the version id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,

    /// Kept alongside the data so the graph never has to rebuild versions.
    #[serde(default)]
    pub preview: String,
}

/// Milliseconds since the Unix epoch, for stamping a new version.
fn now_millis() -> Option<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}

/// First non-empty line of a version, trimmed and capped, so the graph can
/// show what a version says without shipping the whole version over IPC.
fn preview_of(content: &str) -> String {
    const MAX_CHARS: usize = 64;
    let line = content
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if line.chars().count() <= MAX_CHARS {
        return line.to_string();
    }
    let mut out: String = line.chars().take(MAX_CHARS).collect();
    out.push('\u{2026}');
    out
}

/// The smallest single-span edit turning `old` into `new`. Both cut points
/// land on char boundaries in both strings.
fn delta(old: &str, new: &str) -> NodeData {
    let (a, b) = (old.as_bytes(), new.as_bytes());

    let mut start = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    while !(old.is_char_boundary(start) && new.is_char_boundary(start)) {
        start -= 1;
    }

    let max_end = a.len().min(b.len()) - start;
    let mut end = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(max_end)
        .take_while(|(x, y)| x == y)
        .count();
    while !(old.is_char_boundary(a.len() - end) && new.is_char_boundary(b.len() - end)) {
        end -= 1;
    }

    NodeData::Delta {
        keep_start: start,
        keep_end: end,
        insert: new[start..b.len() - end].to_string(),
    }
}

fn apply(parent: &str, data: &NodeData) -> String {
    match data {
        NodeData::Full(text) => text.clone(),
        NodeData::Delta {
            keep_start,
            keep_end,
            insert,
        } => {
            let mut out = String::with_capacity(keep_start + insert.len() + keep_end);
            out.push_str(&parent[..*keep_start]);
            out.push_str(insert);
            out.push_str(&parent[parent.len() - keep_end..]);
            out
        }
    }
}

// Wrapping usize ensures we don't accidentally mix up integers with indices.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeIndex(pub usize);

// Represents the complete change history for a single file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct History {
    pub nodes: Vec<Node>,
    pub current: Option<NodeIndex>,

    // We skip serializing the redo_stack because it represents a "session" state,
    // not the permanent data structure.
    #[serde(skip, default)]
    redo_stack: Vec<NodeIndex>,
}

impl History {
    // Helper to get children sorted by their creation time (Index ID)
    // Returns: Vec<NodeIndex> with the largest (newest) index first.
    pub fn get_latest_children(&self, node_index: NodeIndex) -> Vec<NodeIndex> {
        if node_index.0 >= self.nodes.len() {
            return Vec::new();
        }

        let mut children = self.nodes[node_index.0].children.clone();
        // Sort by index descending. Since we only append to `nodes`,
        // higher index = created later.
        children.sort_by(|a, b| b.cmp(a));
        children
    }

    /// Rebuilds a version's text from the nearest full copy above it.
    pub fn content_of(&self, index: NodeIndex) -> Option<String> {
        let mut chain = Vec::new();
        let mut at = index;
        loop {
            let node = self.nodes.get(at.0)?;
            chain.push(at);
            match (&node.data, node.parent) {
                (NodeData::Full(_), _) => break,
                (NodeData::Delta { .. }, Some(parent)) => at = parent,
                (NodeData::Delta { .. }, None) => return None,
            }
        }
        let mut text = String::new();
        for at in chain.into_iter().rev() {
            text = apply(&text, &self.nodes[at.0].data);
        }
        Some(text)
    }

    pub fn current_content(&self) -> Option<String> {
        self.content_of(self.current?)
    }

    /// Versions since the last full copy on the way up from `index`, counting `index`.
    fn deltas_above(&self, index: NodeIndex) -> usize {
        let mut count = 0;
        let mut at = Some(index);
        while let Some(i) = at {
            let node = &self.nodes[i.0];
            if matches!(node.data, NodeData::Full(_)) {
                break;
            }
            count += 1;
            at = node.parent;
        }
        count
    }

    /// Appends `content` as a child of the current version and moves to it.
    /// Identical content is ignored.
    fn push(&mut self, content: &str, created_at: Option<i64>) -> bool {
        let parent = self.current;
        let parent_text = parent.and_then(|p| self.content_of(p));
        if parent_text.as_deref() == Some(content) {
            return false;
        }

        let data = match (parent, &parent_text) {
            (Some(p), Some(text)) if self.deltas_above(p) + 1 < KEYFRAME_INTERVAL => {
                match delta(text, content) {
                    // A near-total rewrite: the delta saves nothing.
                    NodeData::Delta { insert, .. } if insert.len() >= content.len() => {
                        NodeData::Full(content.to_string())
                    }
                    d => d,
                }
            }
            _ => NodeData::Full(content.to_string()),
        };

        let new_index = NodeIndex(self.nodes.len());
        self.nodes.push(Node {
            data,
            parent,
            children: Vec::new(),
            created_at,
            preview: preview_of(content),
        });

        // Link parent to this new child
        if let Some(p) = parent {
            self.nodes[p.0].children.push(new_index);
        }

        self.current = Some(new_index);
        // Standard behavior: Clear linear redo stack when branching
        self.redo_stack.clear();
        true
    }
}

/// A node without its content, for drawing the history graph.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSummary {
    pub parent: Option<NodeIndex>,
    pub children: Vec<NodeIndex>,
    pub created_at: Option<i64>,
    /// A short excerpt, never the full version content.
    pub preview: String,
}

/// History shape without version contents. The graph shows only a short
/// excerpt per version; shipping every full version over IPC on each save
/// grows with the history.
#[derive(Clone, Debug, Serialize)]
pub struct HistorySummary {
    pub nodes: Vec<NodeSummary>,
    pub current: Option<NodeIndex>,
}

impl From<&History> for HistorySummary {
    fn from(history: &History) -> Self {
        Self {
            nodes: history
                .nodes
                .iter()
                .map(|n| NodeSummary {
                    parent: n.parent,
                    children: n.children.clone(),
                    created_at: n.created_at,
                    preview: n.preview.clone(),
                })
                .collect(),
            current: history.current,
        }
    }
}

/// The single-file format written by older versions of the app.
mod legacy {
    use super::NodeIndex;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    pub struct Node {
        pub content: String,
        pub parent: Option<NodeIndex>,
        #[serde(default)]
        pub created_at: Option<i64>,
    }

    #[derive(Deserialize)]
    pub struct History {
        pub nodes: Vec<Node>,
        pub current: Option<NodeIndex>,
    }

    #[derive(Deserialize)]
    pub struct UndoTree {
        pub entries: HashMap<String, History>,
    }
}

impl From<legacy::History> for History {
    fn from(old: legacy::History) -> Self {
        // Nodes are only ever appended, so every parent comes before its
        // children and replaying them in order rebuilds the same tree.
        let mut history = History::default();
        for (i, node) in old.nodes.iter().enumerate() {
            history.current = node.parent.filter(|p| p.0 < i);
            history.nodes_push_unchecked(&node.content, node.created_at);
        }
        history.current = old.current.filter(|c| c.0 < history.nodes.len());
        history
    }
}

impl History {
    /// Like `push`, but keeps identical versions so node indices survive migration.
    fn nodes_push_unchecked(&mut self, content: &str, created_at: Option<i64>) {
        if !self.push(content, created_at) {
            let parent = self.current;
            let new_index = NodeIndex(self.nodes.len());
            self.nodes.push(Node {
                data: NodeData::Delta {
                    keep_start: content.len(),
                    keep_end: 0,
                    insert: String::new(),
                },
                parent,
                children: Vec::new(),
                created_at,
                preview: preview_of(content),
            });
            if let Some(p) = parent {
                self.nodes[p.0].children.push(new_index);
            }
            self.current = Some(new_index);
        }
    }
}

/// Writes via a temp file + rename, so a crash mid-write can't leave a
/// truncated file behind.
fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut writer = io::BufWriter::new(fs::File::create(&tmp)?);
        serde_json::to_writer(&mut writer, value).map_err(io::Error::other)?;
        writer.flush()?;
    }
    fs::rename(&tmp, path)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> io::Result<T> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[derive(Debug, Default)]
pub struct UndoTree {
    /// Where histories live. `None` keeps everything in memory (tests, or
    /// when the directory could not be opened).
    dir: Option<PathBuf>,
    /// Writing name -> history id (its file stem).
    index: HashMap<String, String>,
    /// Histories loaded so far, by id.
    loaded: HashMap<String, History>,
    dirty: HashSet<String>,
    index_dirty: bool,
}

impl UndoTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the history directory at `path`, migrating the old single-file
    /// format if that is what is there.
    pub fn load(path: &Path) -> io::Result<Self> {
        let backup = path.with_extension("v1.json");
        if path.is_file() {
            // Move the old file aside first: the directory takes its name,
            // and the data stays safe if the migration below fails midway.
            fs::rename(path, &backup)?;
        }
        fs::create_dir_all(path)?;

        let index_path = path.join(INDEX_FILE);
        if index_path.exists() {
            return Ok(Self {
                dir: Some(path.to_path_buf()),
                index: read_json(&index_path)?,
                ..Self::default()
            });
        }

        let mut tree = Self {
            dir: Some(path.to_path_buf()),
            ..Self::default()
        };
        if backup.is_file() {
            let old: legacy::UndoTree = read_json(&backup)?;
            for (name, history) in old.entries {
                let id = tree.id_for(&name);
                tree.loaded.insert(id.clone(), history.into());
                tree.dirty.insert(id);
            }
        }
        tree.index_dirty = true;
        tree.save()?;
        Ok(tree)
    }

    /// Writes the index and every history changed since the last save.
    pub fn save(&mut self) -> io::Result<()> {
        let Some(dir) = &self.dir else {
            self.dirty.clear();
            self.index_dirty = false;
            return Ok(());
        };
        // Histories before the index, so the index never names a missing file.
        for id in self.dirty.iter() {
            if let Some(history) = self.loaded.get(id) {
                write_json(&dir.join(format!("{id}.json")), history)?;
            }
        }
        self.dirty.clear();
        if self.index_dirty {
            write_json(&dir.join(INDEX_FILE), &self.index)?;
            self.index_dirty = false;
        }
        Ok(())
    }

    /// The id for `name`, assigning a new one if it has no history yet.
    fn id_for(&mut self, name: &str) -> String {
        if let Some(id) = self.index.get(name) {
            return id.clone();
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        self.index.insert(name.to_string(), id.clone());
        self.index_dirty = true;
        id
    }

    /// The history with id `id`, read from disk the first time it is needed.
    fn loaded_mut(&mut self, id: &str) -> &mut History {
        if !self.loaded.contains_key(id) {
            let file = self.dir.as_ref().map(|dir| dir.join(format!("{id}.json")));
            let from_disk = match file {
                Some(file) if file.exists() => read_json(&file)
                    .inspect_err(|e| eprintln!("Could not read history {}: {e}", file.display()))
                    .unwrap_or_default(),
                _ => History::default(),
            };
            self.loaded.insert(id.to_string(), from_disk);
        }
        self.loaded.get_mut(id).expect("inserted above")
    }

    fn history_mut(&mut self, name: &str) -> Option<&mut History> {
        let id = self.index.get(name)?.clone();
        Some(self.loaded_mut(&id))
    }

    fn mark_dirty(&mut self, name: &str) {
        if let Some(id) = self.index.get(name) {
            self.dirty.insert(id.clone());
        }
    }

    /// Adds a change. If content is identical to current state, it is ignored.
    pub fn add_change(&mut self, file_name: &str, content: &str) {
        let id = self.id_for(file_name);
        if self.loaded_mut(&id).push(content, now_millis()) {
            self.dirty.insert(id);
        }
    }

    pub fn rename_entry(&mut self, old_name: &str, new_name: &str) {
        if let Some(id) = self.index.remove(old_name) {
            self.drop_entry(new_name);
            self.index.insert(new_name.to_string(), id);
            self.index_dirty = true;
        }
    }

    /// Re-keys every entry under folder `old_prefix` to live under `new_prefix`
    /// (both without trailing slash), so moving or renaming a folder keeps the
    /// version history of every writing inside it.
    pub fn rename_prefix(&mut self, old_prefix: &str, new_prefix: &str) {
        let old_dir = format!("{old_prefix}/");
        let keys: Vec<String> = self
            .index
            .keys()
            .filter(|k| k.starts_with(&old_dir))
            .cloned()
            .collect();
        for key in keys {
            let rest = &key[old_dir.len()..];
            let new_key = format!("{new_prefix}/{rest}");
            self.rename_entry(&key, &new_key);
        }
    }

    /// Forgets `name`'s history, e.g. when another writing is renamed over it.
    fn drop_entry(&mut self, name: &str) {
        let Some(id) = self.index.remove(name) else {
            return;
        };
        self.index_dirty = true;
        self.loaded.remove(&id);
        self.dirty.remove(&id);
        if let Some(dir) = &self.dir {
            let _ = fs::remove_file(dir.join(format!("{id}.json")));
        }
    }

    pub fn get_history(&mut self, file_name: &str) -> Option<&History> {
        self.history_mut(file_name).map(|h| &*h)
    }

    /// The text of `file_name`'s current version.
    pub fn current_content(&mut self, file_name: &str) -> Option<String> {
        self.get_history(file_name)?.current_content()
    }

    pub fn goto_version(&mut self, file_name: &str, target_node_index: usize) -> Option<String> {
        let history = self.history_mut(file_name)?;
        let target_index = NodeIndex(target_node_index);
        let content = history.content_of(target_index)?;
        history.current = Some(target_index);
        history.redo_stack.clear(); // Moving arbitrarily breaks linear redo
        self.mark_dirty(file_name);
        Some(content)
    }

    pub fn undo(&mut self, file_name: &str) -> Option<String> {
        let history = self.history_mut(file_name)?;
        let current_index = history.current?;
        let parent_index = history.nodes[current_index.0].parent?;

        history.redo_stack.push(current_index);
        history.current = Some(parent_index);
        let content = history.content_of(parent_index);
        self.mark_dirty(file_name);
        content
    }

    pub fn redo_latest_branch(&mut self, file_name: &str) -> Option<String> {
        let history = self.history_mut(file_name)?;

        // 1. Try session redo stack first, 2. otherwise the latest child of
        // the current node.
        let target = match history.redo_stack.pop() {
            Some(redo_index) => redo_index,
            None => *history.get_latest_children(history.current?).first()?,
        };
        history.current = Some(target);
        let content = history.content_of(target);
        self.mark_dirty(file_name);
        content
    }

    // Generate a Graphviz DOT string for visualization
    pub fn to_dot(&mut self, file_name: &str) -> String {
        let history = match self.get_history(file_name) {
            Some(h) => h,
            None => return String::new(),
        };

        let mut dot = String::from("digraph History {\n");
        for (i, node) in history.nodes.iter().enumerate() {
            let label: String = node.preview.chars().take(10).collect();

            let color = if history.current == Some(NodeIndex(i)) {
                "color=red, style=filled, fillcolor=pink"
            } else {
                ""
            };

            dot.push_str(&format!(
                "  node{} [label=\"{}: {}\" {}];\n",
                i, i, label, color
            ));

            if let Some(parent) = node.parent {
                dot.push_str(&format!("  node{} -> node{};\n", parent.0, i));
            }
        }
        dot.push_str("}\n");
        dot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_at(tree: &mut UndoTree, name: &str, i: usize) -> String {
        tree.get_history(name).unwrap().content_of(NodeIndex(i)).unwrap()
    }

    #[test]
    fn test_sort_by_latest_logic() {
        let mut tree = UndoTree::new();
        tree.add_change("file.txt", "Original"); // Node 0

        // Create Branch A
        tree.add_change("file.txt", "Branch A - 1"); // Node 1

        // Go back to root
        tree.undo("file.txt"); // Back to Node 0

        // Create Branch B (Later in time)
        tree.add_change("file.txt", "Branch B - 1"); // Node 2

        // Go back to root
        tree.undo("file.txt"); // Back to Node 0

        let history = tree.get_history("file.txt").unwrap();
        let root_idx = NodeIndex(0);

        // Get children sorted by latest
        let sorted_children = history.get_latest_children(root_idx);

        assert_eq!(sorted_children.len(), 2);
        assert_eq!(sorted_children[0], NodeIndex(2)); // Branch B (Newer) should be first
        assert_eq!(sorted_children[1], NodeIndex(1)); // Branch A (Older) should be second
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history");

        let mut tree = UndoTree::load(&path).unwrap();
        tree.add_change("a.md", "one\n\"quoted\"");
        tree.add_change("a.md", "two");
        tree.save().unwrap();
        tree.add_change("a.md", "three");
        tree.save().unwrap();

        let mut loaded = UndoTree::load(&path).unwrap();
        let history = loaded.get_history("a.md").unwrap();
        assert_eq!(history.nodes.len(), 3);
        assert_eq!(history.current, Some(NodeIndex(2)));
        assert_eq!(history.content_of(NodeIndex(0)).unwrap(), "one\n\"quoted\"");
        assert!(!path.join("index.tmp").exists());

        let summary = HistorySummary::from(history);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&summary).unwrap()).unwrap();
        assert_eq!(json["current"], 2);
        assert_eq!(json["nodes"][0]["parent"], serde_json::Value::Null);
        assert_eq!(json["nodes"][0]["children"], serde_json::json!([1]));
        assert_eq!(json["nodes"][2]["parent"], 1);
        // The summary carries an excerpt and a timestamp, never full content.
        assert_eq!(json["nodes"][0]["preview"], "one");
        assert_eq!(json["nodes"][2]["preview"], "three");
        assert!(json["nodes"][0]["createdAt"].is_i64());
        assert!(json["nodes"][0].get("content").is_none());
    }

    #[test]
    fn test_redo_latest_branch() {
        let mut tree = UndoTree::new();
        tree.add_change("f", "root");
        tree.add_change("f", "old_branch");
        tree.undo("f");
        tree.add_change("f", "new_branch"); // This breaks the linear redo stack

        // We are at "new_branch". Go back to root.
        tree.undo("f");

        // Standard redo might be empty or confused depending on implementation,
        // but redo_latest_branch MUST pick "new_branch" (Node 2) over "old_branch" (Node 1)
        let content = tree.redo_latest_branch("f").unwrap();
        assert_eq!(content, "new_branch");
    }

    #[test]
    fn test_delta_roundtrip() {
        let cases = [
            ("", "hello"),
            ("hello", ""),
            ("hello world", "hello there world"),
            ("aaaa", "aaaaaa"),
            ("abcabc", "abc"),
            ("café au lait", "cafés au lait"),
            ("日本語のテキスト", "日本のテキスト"),
            ("emoji 🙂 here", "emoji 🙃 here"),
            ("same", "same"),
        ];
        for (old, new) in cases {
            assert_eq!(apply(old, &delta(old, new)), new, "{old:?} -> {new:?}");
        }
    }

    #[test]
    fn test_stores_deltas_with_keyframes() {
        let mut tree = UndoTree::new();
        let big = "x".repeat(100_000);
        let versions: Vec<String> = (0..70).map(|i| format!("{big}{i}")).collect();
        for v in &versions {
            tree.add_change("big.md", v);
        }

        let history = tree.get_history("big.md").unwrap();
        let full = history
            .nodes
            .iter()
            .filter(|n| matches!(n.data, NodeData::Full(_)))
            .count();
        assert_eq!(full, 3); // versions 0, 32 and 64
        for (i, v) in versions.iter().enumerate() {
            assert_eq!(&history.content_of(NodeIndex(i)).unwrap(), v);
        }
    }

    #[test]
    fn test_branches_rebuild_from_their_own_parent() {
        let mut tree = UndoTree::new();
        tree.add_change("f", "one two three");
        tree.add_change("f", "one 2 three");
        tree.goto_version("f", 0);
        tree.add_change("f", "one two 3");
        assert_eq!(content_at(&mut tree, "f", 1), "one 2 three");
        assert_eq!(content_at(&mut tree, "f", 2), "one two 3");
        assert_eq!(tree.current_content("f").unwrap(), "one two 3");
    }

    #[test]
    fn test_save_writes_only_changed_histories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history");
        let mut tree = UndoTree::load(&path).unwrap();
        tree.add_change("a.md", "a");
        tree.add_change("b.md", "b");
        tree.save().unwrap();

        let id_b = tree.index["b.md"].clone();
        let b_file = path.join(format!("{id_b}.json"));
        fs::remove_file(&b_file).unwrap();

        tree.add_change("a.md", "a2");
        tree.save().unwrap();
        assert!(!b_file.exists());
    }

    #[test]
    fn test_rename_survives_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history");
        let mut tree = UndoTree::load(&path).unwrap();
        tree.add_change("old/a.md", "a");
        tree.add_change("old/b.md", "b");
        tree.add_change("c.md", "c");
        tree.rename_prefix("old", "new");
        tree.rename_entry("c.md", "d.md");
        tree.save().unwrap();

        let mut loaded = UndoTree::load(&path).unwrap();
        assert_eq!(loaded.current_content("new/a.md").unwrap(), "a");
        assert_eq!(loaded.current_content("new/b.md").unwrap(), "b");
        assert_eq!(loaded.current_content("d.md").unwrap(), "c");
        assert!(loaded.get_history("old/a.md").is_none());
        assert!(loaded.get_history("c.md").is_none());
    }

    #[test]
    fn test_rename_over_existing_drops_its_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history");
        let mut tree = UndoTree::load(&path).unwrap();
        tree.add_change("a.md", "a");
        tree.add_change("b.md", "b");
        tree.save().unwrap();
        let id_b = tree.index["b.md"].clone();

        tree.rename_entry("a.md", "b.md");
        tree.save().unwrap();
        assert_eq!(tree.current_content("b.md").unwrap(), "a");
        assert!(!path.join(format!("{id_b}.json")).exists());
    }

    #[test]
    fn test_migrates_single_file_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history");
        let legacy = serde_json::json!({
            "entries": {
                "a.md": {
                    "nodes": [
                        { "content": "root", "parent": null, "children": [1, 2], "created_at": 5 },
                        { "content": "root plus", "parent": 0, "children": [] },
                        { "content": "root", "parent": 0, "children": [] }
                    ],
                    "current": 1
                }
            }
        });
        fs::write(&path, legacy.to_string()).unwrap();

        let mut tree = UndoTree::load(&path).unwrap();
        assert!(path.is_dir());
        assert!(path.with_extension("v1.json").is_file());

        let history = tree.get_history("a.md").unwrap();
        assert_eq!(history.nodes.len(), 3);
        assert_eq!(history.current, Some(NodeIndex(1)));
        assert_eq!(history.nodes[0].children, vec![NodeIndex(1), NodeIndex(2)]);
        assert_eq!(history.nodes[0].created_at, Some(5));
        assert_eq!(history.nodes[1].preview, "root plus");
        assert_eq!(content_at(&mut tree, "a.md", 1), "root plus");
        assert_eq!(content_at(&mut tree, "a.md", 2), "root");

        // Reopening reads the new layout and leaves the backup alone.
        let mut again = UndoTree::load(&path).unwrap();
        assert_eq!(again.current_content("a.md").unwrap(), "root plus");
    }

    #[test]
    fn test_resumes_interrupted_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history");
        let legacy = serde_json::json!({
            "entries": { "a.md": { "nodes": [ { "content": "x", "parent": null, "children": [] } ], "current": 0 } }
        });
        // Crashed after moving the old file aside, before writing the index.
        fs::write(path.with_extension("v1.json"), legacy.to_string()).unwrap();
        fs::create_dir_all(&path).unwrap();

        let mut tree = UndoTree::load(&path).unwrap();
        assert_eq!(tree.current_content("a.md").unwrap(), "x");
    }
}
