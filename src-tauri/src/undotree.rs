use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct History {
    snapshots: Vec<String>,
    current_index: usize,
}

#[derive(Debug, Default)]
pub struct UndoTree {
    entries: HashMap<String, History>,
}

impl UndoTree {
    pub fn new() -> Self {
        Self::default()
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
mod tests {
    use super::*;

    #[test]
    fn test_add_and_basic_undo_redo() {
        let mut tree = UndoTree::new();
        let file = "test.lexical";

        tree.add_change(file, "version 1");
        tree.add_change(file, "version 2");
        tree.add_change(file, "version 3");

        assert_eq!(tree.undo(file), Some("version 2".to_string()));
        assert_eq!(tree.undo(file), Some("version 1".to_string()));
        assert_eq!(tree.undo(file), None);

        assert_eq!(tree.redo(file), Some("version 2".to_string()));
        assert_eq!(tree.redo(file), Some("version 3".to_string()));
        assert_eq!(tree.redo(file), None);
    }

    #[test]
    fn test_new_change_truncates_redo_history() {
        let mut tree = UndoTree::new();
        let file = "test.lexical";

        tree.add_change(file, "v1");
        tree.add_change(file, "v2");
        tree.add_change(file, "v3");

        tree.undo(file); // Current is "v2"

        tree.add_change(file, "v4"); // This should eliminate "v3"

        assert_eq!(tree.redo(file), None); // Cannot redo to "v3"
        assert_eq!(tree.undo(file), Some("v2".to_string()));
    }

    #[test]
    fn test_no_new_entry_for_identical_content() {
        let mut tree = UndoTree::new();
        let file = "test.lexical";

        tree.add_change(file, "v1");
        tree.add_change(file, "v1"); // Identical, should be ignored

        assert_eq!(tree.undo(file), None); // Only one "v1" state exists

        tree.add_change(file, "v2");
        tree.add_change(file, "v2"); // Identical, should be ignored

        assert_eq!(tree.undo(file), Some("v1".to_string()));
    }

    #[test]
    fn test_undo_redo_boundary_conditions() {
        let mut tree = UndoTree::new();
        let file = "test.lexical";

        assert_eq!(tree.undo(file), None);
        assert_eq!(tree.redo(file), None);

        tree.add_change(file, "v1");
        assert_eq!(tree.undo(file), None);
        assert_eq!(tree.redo(file), None);
    }

    #[test]
    fn test_history_is_isolated_between_files() {
        let mut tree = UndoTree::new();
        let file1 = "file1.lexical";
        let file2 = "file2.lexical";

        tree.add_change(file1, "f1v1");
        tree.add_change(file2, "f2v1");
        tree.add_change(file1, "f1v2");

        assert_eq!(tree.undo(file1), Some("f1v1".to_string()));
        assert_eq!(tree.undo(file2), None); // file2 is unaffected
        assert_eq!(tree.redo(file1), Some("f1v2".to_string()));
    }

    #[test]
    fn test_clear_removes_history() {
        let mut tree = UndoTree::new();
        let file = "test.lexical";

        tree.add_change(file, "v1");
        tree.add_change(file, "v2");

        tree.clear(file);

        assert_eq!(tree.undo(file), None);
        assert_eq!(tree.redo(file), None);
    }
}
