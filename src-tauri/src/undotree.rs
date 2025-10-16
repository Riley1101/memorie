use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct History {
    snapshots: Vec<String>,
    current_index: usize,
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

        if history.current_index + 1 < history.snapshots.len() {
            history.snapshots.truncate(history.current_index + 1);
        }

        if let Some(last_snapshot) = history.snapshots.get(history.current_index) {
            if last_snapshot == content && !history.snapshots.is_empty() {
                return;
            }
        }

        history.snapshots.push(content.to_string());
        history.current_index = history.snapshots.len() - 1;
    }

    pub fn undo(&mut self, file_name: &str) -> Option<String> {
        if let Some(history) = self.entries.get_mut(file_name) {
            if history.current_index > 0 {
                history.current_index -= 1;
                return Some(history.snapshots[history.current_index].clone());
            }
        }
        None
    }

    pub fn redo(&mut self, file_name: &str) -> Option<String> {
        if let Some(history) = self.entries.get_mut(file_name) {
            if history.current_index + 1 < history.snapshots.len() {
                history.current_index += 1;
                return Some(history.snapshots[history.current_index].clone());
            }
        }
        None
    }

    pub fn clear(&mut self, file_name: &str) {
        self.entries.remove(file_name);
    }
}

#[cfg(test)]
mod undotree_tests {
    use tempfile::tempdir;

    use super::UndoTree;
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
        assert_eq!(history.snapshots.len(), 1);
        assert_eq!(history.current_index, 0);
        assert_eq!(history.snapshots[0], "first version");
    }

    #[test]
    fn test_add_multiple_changes() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "second");
        tree.add_change("file1.txt", "third");

        let history = tree.entries.get("file1.txt").unwrap();
        assert_eq!(history.snapshots.len(), 3);
        assert_eq!(history.current_index, 2);
        assert_eq!(history.snapshots[2], "third");
    }

    #[test]
    fn test_add_same_content_is_ignored() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "first"); // This should be ignored

        let history = tree.entries.get("file1.txt").unwrap();
        assert_eq!(history.snapshots.len(), 1);
        assert_eq!(history.current_index, 0);
    }

    #[test]
    fn test_undo_and_redo_simple() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "second");

        let undone_content = tree.undo("file1.txt").unwrap();
        assert_eq!(undone_content, "first");

        let redone_content = tree.redo("file1.txt").unwrap();
        assert_eq!(redone_content, "second");
    }

    #[test]
    fn test_undo_past_beginning_returns_none() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.undo("file1.txt");

        assert!(tree.undo("file1.txt").is_none());
    }

    #[test]
    fn test_redo_without_undo_returns_none() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "first");
        tree.add_change("file1.txt", "second");

        // No undo has been performed, so redo should be None
        assert!(tree.redo("file1.txt").is_none());
    }

    #[test]
    fn test_add_change_after_undo_truncates_history() {
        let mut tree = UndoTree::new();
        tree.add_change("file1.txt", "a");
        tree.add_change("file1.txt", "b");
        tree.add_change("file1.txt", "c");

        tree.undo("file1.txt");

        tree.add_change("file1.txt", "d");

        let history = tree.entries.get("file1.txt").unwrap();
        assert_eq!(history.snapshots.len(), 3);
        assert_eq!(history.current_index, 2);
        assert_eq!(
            history.snapshots,
            vec!["a".to_string(), "b".to_string(), "d".to_string()]
        );

        assert!(tree.redo("file1.txt").is_none());
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
        tree_to_save.add_change("file1.txt", "hello");
        tree_to_save.add_change("file1.txt", "world");
        tree_to_save.add_change("file2.txt", "test");
        tree_to_save.undo("file1.txt");

        tree_to_save.save(&file_path)?;

        let loaded_tree = UndoTree::load(&file_path)?;

        assert_eq!(tree_to_save.entries.len(), loaded_tree.entries.len());

        let history1_saved = tree_to_save.entries.get("file1.txt").unwrap();
        let history1_loaded = loaded_tree.entries.get("file1.txt").unwrap();
        assert_eq!(history1_saved.snapshots, history1_loaded.snapshots);
        assert_eq!(history1_saved.current_index, history1_loaded.current_index);

        let history2_saved = tree_to_save.entries.get("file2.txt").unwrap();
        let history2_loaded = loaded_tree.entries.get("file2.txt").unwrap();
        assert_eq!(history2_saved.snapshots, history2_loaded.snapshots);
        assert_eq!(history2_saved.current_index, history2_loaded.current_index);

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
