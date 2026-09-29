//! Blocks: the document's structure.
//!
//! Every block carries a [`BlockId`], including blocks nested inside a quote
//! or a list item, so a selection or an annotation can name any block in the
//! document without a path.

use crate::ids::BlockId;
use crate::inline::{ImageNode, InlineContent};
use serde::{Deserialize, Serialize};

/// One block, with its identity separated from its content so an id survives
/// a change of kind (paragraph → heading keeps the same block).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    #[serde(flatten)]
    pub kind: BlockKind,
}

impl Block {
    pub fn new(id: BlockId, kind: BlockKind) -> Self {
        Self { id, kind }
    }

    /// The block's own inline content, for the kinds that have any.
    pub fn content(&self) -> Option<&InlineContent> {
        match &self.kind {
            BlockKind::Paragraph { content } | BlockKind::Heading { content, .. } => Some(content),
            _ => None,
        }
    }

    pub fn content_mut(&mut self) -> Option<&mut InlineContent> {
        match &mut self.kind {
            BlockKind::Paragraph { content } | BlockKind::Heading { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Blocks nested directly inside this one, in document order.
    pub fn children(&self) -> Vec<&Block> {
        match &self.kind {
            BlockKind::Quote { blocks } => blocks.iter().collect(),
            BlockKind::List { items, .. } => items.iter().flat_map(|i| i.blocks.iter()).collect(),
            BlockKind::Table { table } => table
                .rows
                .iter()
                .flat_map(|row| row.cells.iter())
                .flat_map(|cell| cell.blocks.iter())
                .collect(),
            _ => Vec::new(),
        }
    }

    fn children_mut(&mut self) -> Vec<&mut Block> {
        match &mut self.kind {
            BlockKind::Quote { blocks } => blocks.iter_mut().collect(),
            BlockKind::List { items, .. } => {
                items.iter_mut().flat_map(|i| i.blocks.iter_mut()).collect()
            }
            BlockKind::Table { table } => table
                .rows
                .iter_mut()
                .flat_map(|row| row.cells.iter_mut())
                .flat_map(|cell| cell.blocks.iter_mut())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// This block and everything nested inside it, outermost first.
    pub fn descendants(&self) -> Vec<&Block> {
        let mut out = vec![self];
        for child in self.children() {
            out.extend(child.descendants());
        }
        out
    }

    /// The block with id `id`, looked up anywhere inside this subtree.
    ///
    /// A walk rather than a search over a collected list: lookups happen on
    /// every keystroke, several times, and collecting the document into a `Vec`
    /// to find one block is an allocation per lookup.
    pub fn find(&self, id: BlockId) -> Option<&Block> {
        if self.id == id {
            return Some(self);
        }
        self.children().into_iter().find_map(|child| child.find(id))
    }

    /// The block with id `id`, looked up anywhere inside this subtree.
    pub fn find_mut(&mut self, id: BlockId) -> Option<&mut Block> {
        if self.id == id {
            return Some(self);
        }
        for child in self.children_mut() {
            if let Some(found) = child.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Plain text of the block's own content, without its children.
    pub fn own_text(&self) -> String {
        match &self.kind {
            BlockKind::Paragraph { content } | BlockKind::Heading { content, .. } => {
                content.plain_text()
            }
            BlockKind::CodeBlock { code, .. } => code.clone(),
            BlockKind::Image { image } => image.alt.clone(),
            BlockKind::Html { html } => html.clone(),
            BlockKind::Quote { .. } | BlockKind::List { .. } | BlockKind::Table { .. } => {
                String::new()
            }
            BlockKind::ThematicBreak => String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BlockKind {
    Paragraph {
        content: InlineContent,
    },
    Heading {
        /// 1–6.
        level: u8,
        content: InlineContent,
    },
    /// Code is text, not inline content: nothing inside it is marked up.
    CodeBlock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        code: String,
    },
    Quote {
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        /// First number of an ordered list, when it doesn't start at 1.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<u64>,
        /// A tight list has no blank lines between items; loose items are
        /// rendered as paragraphs. Kept so round-tripping doesn't reflow lists.
        #[serde(default = "default_true")]
        tight: bool,
        items: Vec<ListItem>,
    },
    /// An image on its own, the way Markdown writes a figure.
    Image {
        image: ImageNode,
    },
    Table {
        table: TableNode,
    },
    ThematicBreak,
    /// A raw HTML block, kept verbatim so a document that uses HTML survives a
    /// round-trip instead of being silently dropped.
    Html {
        html: String,
    },
}

fn default_true() -> bool {
    true
}

impl BlockKind {
    pub fn paragraph(content: impl Into<InlineContent>) -> Self {
        BlockKind::Paragraph {
            content: content.into(),
        }
    }

    pub fn heading(level: u8, content: impl Into<InlineContent>) -> Self {
        BlockKind::Heading {
            level: level.clamp(1, 6),
            content: content.into(),
        }
    }

    /// A short, stable name for the frontend and for tests.
    pub fn name(&self) -> &'static str {
        match self {
            BlockKind::Paragraph { .. } => "paragraph",
            BlockKind::Heading { .. } => "heading",
            BlockKind::CodeBlock { .. } => "codeBlock",
            BlockKind::Quote { .. } => "quote",
            BlockKind::List { .. } => "list",
            BlockKind::Image { .. } => "image",
            BlockKind::Table { .. } => "table",
            BlockKind::ThematicBreak => "thematicBreak",
            BlockKind::Html { .. } => "html",
        }
    }

    /// Whether the kind holds editable inline content at all.
    pub fn is_textual(&self) -> bool {
        matches!(
            self,
            BlockKind::Paragraph { .. } | BlockKind::Heading { .. }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    /// `Some` for a task list item: whether it is ticked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    pub blocks: Vec<Block>,
}

impl ListItem {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self {
            checked: None,
            blocks,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnAlignment {
    #[default]
    None,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableNode {
    /// Per-column alignment from the delimiter row.
    #[serde(default)]
    pub alignments: Vec<ColumnAlignment>,
    /// The header row first, then the body rows.
    pub rows: Vec<TableRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// A cell holds blocks like any other container, so a cell containing a list
/// or two paragraphs needs no separate model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCell {
    pub blocks: Vec<Block>,
}
