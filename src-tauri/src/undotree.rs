use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

// Represents a single state in the history tree.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    pub content: String,
    pub parent: Option<NodeIndex>,
    pub children: Vec<NodeIndex>,

    // Milliseconds since the epoch. Histories written before this field
    // existed have no timestamp, so the graph falls back to the version id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
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
                    preview: preview_of(&n.content),
                })
                .collect(),
            current: history.current,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UndoTree {
    entries: HashMap<String, History>,
}

impl UndoTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(path)?;
        let tree = serde_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(tree)
    }

    /// Written compact (every autosave rewrites the whole tree) and via a temp
    /// file + rename, so a crash mid-write can't leave truncated history.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let tmp = path.with_extension("tmp");
        {
            let mut writer = io::BufWriter::new(fs::File::create(&tmp)?);
            serde_json::to_writer(&mut writer, self).map_err(io::Error::other)?;
            writer.flush()?;
        }
        fs::rename(&tmp, path)
    }

    /// Adds a change. If content is identical to current state, it is ignored.
    pub fn add_change(&mut self, file_name: &str, content: &str) {
        let history = self.entries.entry(file_name.to_string()).or_default();

        // 1. Deduplication check: Don't add if identical to current
        if let Some(current_index) = history.current {
            if history.nodes[current_index.0].content == content {
                return;
            }
        }

        let new_index = NodeIndex(history.nodes.len());

        let new_node = Node {
            content: content.to_string(),
            parent: history.current,
            children: Vec::new(),
            created_at: now_millis(),
        };

        history.nodes.push(new_node);

        // Link parent to this new child
        if let Some(current_index) = history.current {
            history.nodes[current_index.0].children.push(new_index);
        }

        // Update state
        history.current = Some(new_index);

        // Standard behavior: Clear linear redo stack when branching
        history.redo_stack.clear();
    }

    pub fn rename_entry(&mut self, old_name: &str, new_name: &str) {
        if let Some(history) = self.entries.remove(old_name) {
            self.entries.insert(new_name.to_string(), history);
        }
    }

    /// Re-keys every entry under folder `old_prefix` to live under `new_prefix`
    /// (both without trailing slash), so moving or renaming a folder keeps the
    /// version history of every writing inside it.
    pub fn rename_prefix(&mut self, old_prefix: &str, new_prefix: &str) {
        let old_dir = format!("{old_prefix}/");
        let keys: Vec<String> = self
            .entries
            .keys()
            .filter(|k| k.starts_with(&old_dir))
            .cloned()
            .collect();
        for key in keys {
            let rest = &key[old_dir.len()..];
            let new_key = format!("{new_prefix}/{rest}");
            if let Some(history) = self.entries.remove(&key) {
                self.entries.insert(new_key, history);
            }
        }
    }

    pub fn get_history(&self, file_name: &str) -> Option<&History> {
        self.entries.get(file_name)
    }

    pub fn goto_version(&mut self, file_name: &str, target_node_index: usize) -> Option<&str> {
        let history = self.entries.get_mut(file_name)?;
        if target_node_index >= history.nodes.len() {
            return None;
        }

        let target_index = NodeIndex(target_node_index);
        history.current = Some(target_index);
        history.redo_stack.clear(); // Moving arbitrarily breaks linear redo

        Some(history.nodes[target_index.0].content.as_str())
    }

    pub fn undo(&mut self, file_name: &str) -> Option<&str> {
        let history = self.entries.get_mut(file_name)?;
        let current_index = history.current?;
        let parent_index = history.nodes[current_index.0].parent?;

        history.redo_stack.push(current_index);
        history.current = Some(parent_index);

        Some(history.nodes[parent_index.0].content.as_str())
    }

    pub fn redo_latest_branch(&mut self, file_name: &str) -> Option<&str> {
        let history = self.entries.get_mut(file_name)?;

        // 1. Try session redo stack first
        if let Some(redo_index) = history.redo_stack.pop() {
            history.current = Some(redo_index);
            return Some(history.nodes[redo_index.0].content.as_str());
        }

        // 2. Otherwise, look for the latest child of current node
        let current_index = history.current?;
        let latest_children = history.get_latest_children(current_index);

        if let Some(&latest_child) = latest_children.first() {
            history.current = Some(latest_child);
            return Some(history.nodes[latest_child.0].content.as_str());
        }

        None
    }

    // Generate a Graphviz DOT string for visualization
    pub fn to_dot(&self, file_name: &str) -> String {
        let history = match self.entries.get(file_name) {
            Some(h) => h,
            None => return String::new(),
        };

        let mut dot = String::from("digraph History {\n");
        for (i, node) in history.nodes.iter().enumerate() {
            let label = if node.content.len() > 10 {
                format!("{}...", &node.content[0..10])
            } else {
                node.content.clone()
            };

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

        let history = tree.entries.get("file.txt").unwrap();
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

        let mut tree = UndoTree::new();
        tree.add_change("a.md", "one\n\"quoted\"");
        tree.add_change("a.md", "two");
        tree.save(&path).unwrap();
        tree.add_change("a.md", "three");
        tree.save(&path).unwrap();

        let loaded = UndoTree::load(&path).unwrap();
        let history = loaded.get_history("a.md").unwrap();
        assert_eq!(history.nodes.len(), 3);
        assert_eq!(history.current, Some(NodeIndex(2)));
        assert_eq!(history.nodes[0].content, "one\n\"quoted\"");
        assert!(!path.with_extension("tmp").exists());

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
}
