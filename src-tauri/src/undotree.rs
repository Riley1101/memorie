use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

// Represents a single state in the history tree.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Node {
    content: String,
    // The parent node (the state we came from).
    // `None` for the root node.
    parent: Option<NodeIndex>,
    // Child nodes (states that come after this one).
    // Can be more than one, creating branches.
    children: Vec<NodeIndex>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
struct NodeIndex(usize);

// Represents the complete change history for a single file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    // All nodes (states) are stored in an arena-like vector.
    nodes: Vec<Node>,
    // Points to the current state's index in `nodes`.
    current: Option<NodeIndex>,
    // A transient stack to manage linear redo.
    // We `skip` serializing this, as it's runtime state.
    #[serde(skip)]
    redo_stack: Vec<NodeIndex>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            current: None,
            redo_stack: Vec::new(),
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

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, data)
    }

    pub fn add_change(&mut self, file_name: &str, content: &str) {
        let history = self.entries.entry(file_name.to_string()).or_default();

        // Don't add if content is identical to the current state.
        if let Some(current_index) = history.current {
            if history.nodes[current_index.0].content == content {
                return;
            }
        }

        // Create the new node
        let new_node = Node {
            content: content.to_string(),
            parent: history.current, // Parent is the current node
            children: Vec::new(),
        };

        // Add node to the arena
        let new_index = NodeIndex(history.nodes.len());
        history.nodes.push(new_node);

        // Link this new node as a child of its parent
        if let Some(current_index) = history.current {
            history.nodes[current_index.0].children.push(new_index);
        }

        // The new node is now the current state
        history.current = Some(new_index);

        // A new change invalidates the old redo stack.
        history.redo_stack.clear();
    }

    pub fn get_history(&self, file_name: &str) -> Option<&History> {
        self.entries.get(file_name)
    }

    pub fn undo(&mut self, file_name: &str) -> Option<String> {
        if let Some(history) = self.entries.get_mut(file_name) {
            if let Some(current_index) = history.current {
                let current_node = &history.nodes[current_index.0];

                // Check if a parent exists
                if let Some(parent_index) = current_node.parent {
                    // Move current pointer to the parent
                    history.current = Some(parent_index);
                    // Push the node we *just left* onto the redo stack
                    history.redo_stack.push(current_index);
                    // Return the parent's content
                    return Some(history.nodes[parent_index.0].content.clone());
                }
            }
        }
        // No current node or no parent node, can't undo
        None
    }

    pub fn redo(&mut self, file_name: &str) -> Option<String> {
        if let Some(history) = self.entries.get_mut(file_name) {
            // Check if there's anything on the redo stack
            if let Some(redo_index) = history.redo_stack.pop() {
                let node_to_redo_to = &history.nodes[redo_index.0];

                // Check if the node to redo to is a valid child of current
                if node_to_redo_to.parent == history.current {
                    // It's valid. Move current pointer forward.
                    history.current = Some(redo_index);
                    return Some(node_to_redo_to.content.clone());
                } else {
                    // Invalid stack (e.g., branch changed). Clear it.
                    // Push the popped item back, since it was invalid.
                    history.redo_stack.push(redo_index);
                    history.redo_stack.clear();
                    return None;
                }
            }
        }
        // Redo stack is empty
        None
    }

    pub fn clear(&mut self, file_name: &str) {
        self.entries.remove(file_name);
    }
}

#[cfg(test)]
mod undotree_tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use std::io;

    #[test]
    fn test_new_undo_tree_is_empty() {
        let tree = UndoTree::new();
        assert!(tree.entries.is_empty());
    }

    #[test]
    fn test_add_first_change() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first version");

        let history = tree.entries.get("file1.txt").unwrap();
        assert_eq!(history.nodes.len(), 1);
        assert_eq!(history.current, Some(NodeIndex(0)));
        assert_eq!(history.nodes[0].content, "first version");
        assert_eq!(history.nodes[0].parent, None);
    }

    #[test]
    fn test_add_multiple_changes() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "second");
        tree.add_change("file1.txt", "third");

        let history = tree.entries.get("file1.txt").unwrap();
        assert_eq!(history.nodes.len(), 3);
        assert_eq!(history.current, Some(NodeIndex(2)));
        assert_eq!(history.nodes[2].content, "third");
        assert_eq!(history.nodes[2].parent, Some(NodeIndex(1)));
        assert!(history.nodes[1].children.contains(&NodeIndex(2)));
    }

    #[test]
    fn test_add_same_content_is_ignored() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "first"); // This should be ignored

        let history = tree.entries.get("file1.txt").unwrap();
        assert_eq!(history.nodes.len(), 1);
        assert_eq!(history.current, Some(NodeIndex(0)));
    }

    #[test]
    fn test_undo_and_redo_simple() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "second");

        // State is "second", current=1
        let undone_content = tree.undo("file1.txt").unwrap();
        // State is "first", current=0, redo_stack=[1]
        assert_eq!(undone_content, "first");
        assert_eq!(tree.entries.get("file1.txt").unwrap().current, Some(NodeIndex(0)));

        let redone_content = tree.redo("file1.txt").unwrap();
        // State is "second", current=1, redo_stack=[]
        assert_eq!(redone_content, "second");
        assert_eq!(tree.entries.get("file1.txt").unwrap().current, Some(NodeIndex(1)));
    }

    #[test]
    fn test_undo_past_beginning_returns_none() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "second");

        // State is "second", current=1
        tree.undo("file1.txt"); // State is "first", current=0

        // Try to undo past the root
        assert!(tree.undo("file1.txt").is_none());
        // State is still "first", current=0
        assert_eq!(tree.entries.get("file1.txt").unwrap().current, Some(NodeIndex(0)));
    }

    #[test]
    fn test_redo_without_undo_returns_none() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "second");

        // No undo has been performed, so redo should be None
        assert!(tree.redo("file1.txt").is_none());
        assert_eq!(tree.entries.get("file1.txt").unwrap().current, Some(NodeIndex(1)));
    }

    #[test]
    fn test_add_change_after_undo_creates_branch() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "a"); // Node 0
        tree.add_change("file1.txt", "b"); // Node 1
        tree.add_change("file1.txt", "c"); // Node 2

        // State is "c", current=2
        let undone_content = tree.undo("file1.txt").unwrap(); // State is "b", current=1, redo=[2]
        assert_eq!(undone_content, "b");

        // Add a new change. This creates a branch from "b".
        tree.add_change("file1.txt", "d"); // Node 3, parent=1. redo_stack is cleared.

        let history = tree.entries.get("file1.txt").unwrap();
        assert_eq!(history.nodes.len(), 4); // Nodes 0, 1, 2, 3
        assert_eq!(history.current, Some(NodeIndex(3))); // Current is "d"
        assert_eq!(history.nodes[3].content, "d");
        assert_eq!(history.nodes[3].parent, Some(NodeIndex(1))); // Parent is "b"

        // Node 1 ("b") should now have two children: "c" and "d"
        assert_eq!(history.nodes[1].children.len(), 2);
        assert!(history.nodes[1].children.contains(&NodeIndex(2))); // old "c"
        assert!(history.nodes[1].children.contains(&NodeIndex(3))); // new "d"

        // Redo stack was cleared, so redo does nothing
        assert!(tree.redo("file1.txt").is_none());

        // We can undo back to "b"
        let undone_to_b = tree.undo("file1.txt").unwrap();
        assert_eq!(undone_to_b, "b");
        // And redo back to "d"
        let redone_to_d = tree.redo("file1.txt").unwrap();
        assert_eq!(redone_to_d, "d");
    }

    #[test]
    fn test_clear_removes_history() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "some content");
        assert!(tree.entries.contains_key("file1.txt"));

        tree.clear("file1.txt");
        assert!(!tree.entries.contains_key("file1.txt"));
    }

    #[test]
    fn test_save_and_load() -> io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("undo_tree.json");

        let mut tree_to_save = UndoTree::new();
        tree_to_save.add_change("file1.txt", "hello"); // Node 0
        tree_to_save.add_change("file1.txt", "world"); // Node 1
        tree_to_save.add_change("file2.txt", "test");  // Node 0 (file 2)
        tree_to_save.undo("file1.txt"); // file1: current=0, redo_stack=[1]

        tree_to_save.save(&file_path)?;

        let loaded_tree = UndoTree::load(&file_path)?;

        assert_eq!(tree_to_save.entries.len(), loaded_tree.entries.len());

        // Compare file1 history
        let history1_saved = tree_to_save.entries.get("file1.txt").unwrap();
        let history1_loaded = loaded_tree.entries.get("file1.txt").unwrap();

        // Nodes and current state should be saved
        assert_eq!(history1_saved.nodes, history1_loaded.nodes);
        assert_eq!(history1_saved.current, history1_loaded.current);
        assert_eq!(history1_loaded.current, Some(NodeIndex(0)));

        // Redo stack is transient and should be empty after load
        assert!(history1_loaded.redo_stack.is_empty());
        assert!(!history1_saved.redo_stack.is_empty()); // Saved tree still has it in memory

        // Compare file2 history
        let history2_saved = tree_to_save.entries.get("file2.txt").unwrap();
        let history2_loaded = loaded_tree.entries.get("file2.txt").unwrap();
        assert_eq!(history2_saved.nodes, history2_loaded.nodes);
        assert_eq!(history2_saved.current, history2_loaded.current);
        assert_eq!(history2_loaded.current, Some(NodeIndex(0)));


        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_load_non_existent_file() -> io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("non_existent.json");
        let tree = UndoTree::load(&file_path)?;
        assert!(tree.entries.is_empty());
        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_multiple_branches_from_one_node() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "a"); // Node 0
        tree.add_change("file1.txt", "b"); // Node 1 (child of 0)

        // Undo back to "a"
        tree.undo("file1.txt"); // Current=0, redo_stack=[1]

        // Create first branch
        tree.add_change("file1.txt", "c"); // Node 2 (child of 0)

        // Undo back to "a"
        tree.undo("file1.txt"); // Current=0, redo_stack=[2]

        // Create second branch
        tree.add_change("file1.txt", "d"); // Node 3 (child of 0)

        // Check the tree structure
        let history = tree.entries.get("file1.txt").unwrap();
        assert_eq!(history.nodes.len(), 4); // a, b, c, d
        assert_eq!(history.current, Some(NodeIndex(3))); // Current is "d"

        // Get the parent node "a" (Node 0)
        let parent_node = &history.nodes[0];
        assert_eq!(parent_node.children.len(), 3);
        assert!(parent_node.children.contains(&NodeIndex(1))); // "b"
        assert!(parent_node.children.contains(&NodeIndex(2))); // "c"
        assert!(parent_node.children.contains(&NodeIndex(3))); // "d"

        // Check parents are correct
        assert_eq!(history.nodes[1].parent, Some(NodeIndex(0))); // b -> a
        assert_eq!(history.nodes[2].parent, Some(NodeIndex(0))); // c -> a
        assert_eq!(history.nodes[3].parent, Some(NodeIndex(0))); // d -> a

        // Test linear redo to the last branch
        tree.undo("file1.txt"); // Back to "a", current=0, redo_stack=[3]
        let redone_content = tree.redo("file1.txt").unwrap();
        assert_eq!(redone_content, "d"); // Redo goes to "d"
    }

    #[test]
    fn test_load_corrupted_file() -> io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("corrupted.json");
        fs::write(&file_path, "this is not valid json")?;

        let result = UndoTree::load(&file_path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);

        dir.close()?;
        Ok(())
    }
}