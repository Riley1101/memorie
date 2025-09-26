use std::path::PathBuf;
use std::ops::Range;

/// Represents a location in a source Markdown file.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file_path: PathBuf,
    pub line_range: Range<usize>, // Start and end line numbers
}

/// The data associated with each node in the outliner tree.
#[derive(Debug, Clone)]
pub struct NodeData {
    pub text: String,
    pub location: SourceLocation,
}
