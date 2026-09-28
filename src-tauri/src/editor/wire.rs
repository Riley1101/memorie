//! The wire format between the Svelte UI and the Rust editor.
//!
//! These types are deliberately *not* the engine's own: the frontend depends
//! on this shape, so the document model stays free to change. Two things
//! differ from the engine on purpose.
//!
//! **Offsets are UTF-16 code units here.** That is what a JavaScript string
//! index means, and it is a JavaScript detail, so the engine counts `char`s and
//! the conversion happens at this boundary — the only place where both units
//! are known. An inline image or a raw HTML span counts as one unit on both
//! sides, since the view renders it as a single thing.
//!
//! **Marks are names, not a bitfield.** `["bold", "italic"]` survives a JSON
//! round trip and a version skew; a `u8` does not.

use editor_core::{
    Block, BlockId, BlockKind, ColumnAlignment, Direction, Document, Editor, EditorCommand,
    ImageNode, Inline, InlineContent, MarkSet, Position, SelectionRange,
};
use serde::{Deserialize, Serialize};

// --- offsets ----------------------------------------------------------------

/// The engine offset (in `char`s) for a UTF-16 offset from the frontend.
///
/// An offset that lands *inside* a surrogate pair — which a correct frontend
/// never sends — rounds down to the character it belongs to, rather than
/// stepping over it.
pub fn char_offset(content: &InlineContent, utf16: usize) -> usize {
    let mut chars = 0;
    let mut units = 0;
    for piece in content.pieces() {
        match piece {
            Inline::Text { text, .. } => {
                for c in text.chars() {
                    if units + c.len_utf16() > utf16 {
                        return chars;
                    }
                    units += c.len_utf16();
                    chars += 1;
                }
            }
            // An atom is one unit on both sides.
            _ => {
                if units + 1 > utf16 {
                    return chars;
                }
                units += 1;
                chars += 1;
            }
        }
    }
    chars
}

/// The UTF-16 offset for an engine offset, for sending a selection back.
pub fn utf16_offset(content: &InlineContent, offset: usize) -> usize {
    let mut chars = 0;
    let mut units = 0;
    for piece in content.pieces() {
        if chars >= offset {
            break;
        }
        match piece {
            Inline::Text { text, .. } => {
                for c in text.chars() {
                    if chars >= offset {
                        return units;
                    }
                    units += c.len_utf16();
                    chars += 1;
                }
            }
            _ => {
                units += 1;
                chars += 1;
            }
        }
    }
    units
}

/// The content of `block`, for converting an offset inside it.
fn content_of(document: &Document, block: BlockId) -> Option<&InlineContent> {
    document.block(block).and_then(Block::content)
}

// --- marks ------------------------------------------------------------------

fn mark_names(marks: MarkSet) -> Vec<&'static str> {
    [
        (MarkSet::BOLD, "bold"),
        (MarkSet::ITALIC, "italic"),
        (MarkSet::CODE, "code"),
        (MarkSet::STRIKE, "strike"),
    ]
    .into_iter()
    .filter(|(mark, _)| marks.contains(*mark))
    .map(|(_, name)| name)
    .collect()
}

fn mark_named(name: &str) -> Result<MarkSet, String> {
    match name {
        "bold" | "strong" => Ok(MarkSet::BOLD),
        "italic" | "emphasis" => Ok(MarkSet::ITALIC),
        "code" => Ok(MarkSet::CODE),
        "strike" | "strikethrough" => Ok(MarkSet::STRIKE),
        other => Err(format!("unknown mark '{other}'")),
    }
}

// --- state sent to the frontend ---------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WirePosition {
    pub block: u64,
    /// In UTF-16 code units, so it can index a JavaScript string directly.
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireSelection {
    pub anchor: WirePosition,
    pub head: WirePosition,
}

/// A run of characters sharing formatting. `length` is in UTF-16 units so the
/// view can lay runs out without re-measuring.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireRun {
    #[serde(rename = "type")]
    pub kind: &'static str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub text: String,
    pub length: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Set for an image run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// Raw HTML, for an html run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    pub blocks: Vec<WireBlock>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireRow {
    pub cells: Vec<Vec<WireBlock>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireBlock {
    pub id: u64,
    pub kind: &'static str,
    /// The inline content, for the kinds that hold any.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<WireRun>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// A code block's text, or a raw HTML block's markup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<WireImage>,
    /// Blocks inside a quote.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<WireBlock>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<WireItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<WireRow>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub alignments: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireImage {
    pub source: String,
    pub alt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireHeading {
    pub id: u64,
    pub level: u8,
    pub text: String,
}

/// Everything the UI needs to draw the document once.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireState {
    pub name: String,
    /// Moves on every edit, so the view can tell stale state from current.
    pub revision: u64,
    pub selection: WireSelection,
    pub blocks: Vec<WireBlock>,
    pub outline: Vec<WireHeading>,
    /// What a formatting toolbar should light up.
    pub active_marks: Vec<&'static str>,
    pub word_count: usize,
    pub can_undo: bool,
    pub can_redo: bool,
    /// The front matter block as it will be written, for the metadata panel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<std::collections::BTreeMap<String, String>>,
}

fn alignment_name(alignment: &ColumnAlignment) -> &'static str {
    match alignment {
        ColumnAlignment::None => "none",
        ColumnAlignment::Left => "left",
        ColumnAlignment::Center => "center",
        ColumnAlignment::Right => "right",
    }
}

fn runs_of(content: &InlineContent) -> Vec<WireRun> {
    content
        .pieces()
        .iter()
        .map(|piece| match piece {
            Inline::Text { text, marks, link } => WireRun {
                kind: "text",
                length: text.encode_utf16().count(),
                text: text.clone(),
                marks: mark_names(*marks),
                href: link.as_ref().map(|link| link.url.clone()),
                title: link.as_ref().and_then(|link| link.title.clone()),
                source: None,
                alt: None,
                html: None,
            },
            Inline::Image(image) => WireRun {
                kind: "image",
                text: String::new(),
                length: 1,
                marks: Vec::new(),
                href: None,
                title: image.title.clone(),
                source: Some(image.source.clone()),
                alt: Some(image.alt.clone()),
                html: None,
            },
            Inline::Html(html) => WireRun {
                kind: "html",
                text: String::new(),
                length: 1,
                marks: Vec::new(),
                href: None,
                title: None,
                source: None,
                alt: None,
                html: Some(html.clone()),
            },
            Inline::HardBreak => WireRun {
                kind: "break",
                text: String::new(),
                length: 1,
                marks: Vec::new(),
                href: None,
                title: None,
                source: None,
                alt: None,
                html: None,
            },
        })
        .collect()
}

fn empty_block(id: u64, kind: &'static str) -> WireBlock {
    WireBlock {
        id,
        kind,
        runs: Vec::new(),
        level: None,
        language: None,
        text: None,
        ordered: None,
        start: None,
        tight: None,
        image: None,
        children: Vec::new(),
        items: Vec::new(),
        rows: Vec::new(),
        alignments: Vec::new(),
    }
}

pub fn block_view(block: &Block) -> WireBlock {
    let mut view = empty_block(block.id.0, block.kind.name());
    match &block.kind {
        BlockKind::Paragraph { content } => view.runs = runs_of(content),
        BlockKind::Heading { level, content } => {
            view.level = Some(*level);
            view.runs = runs_of(content);
        }
        BlockKind::CodeBlock { language, code } => {
            view.language = language.clone();
            view.text = Some(code.clone());
        }
        BlockKind::Quote { blocks } => {
            view.children = blocks.iter().map(block_view).collect();
        }
        BlockKind::List {
            ordered,
            start,
            tight,
            items,
        } => {
            view.ordered = Some(*ordered);
            view.start = *start;
            view.tight = Some(*tight);
            view.items = items
                .iter()
                .map(|item| WireItem {
                    checked: item.checked,
                    blocks: item.blocks.iter().map(block_view).collect(),
                })
                .collect();
        }
        BlockKind::Image { image } => {
            view.image = Some(WireImage {
                source: image.source.clone(),
                alt: image.alt.clone(),
                title: image.title.clone(),
            });
        }
        BlockKind::Table { table } => {
            view.alignments = table.alignments.iter().map(alignment_name).collect();
            view.rows = table
                .rows
                .iter()
                .map(|row| WireRow {
                    cells: row
                        .cells
                        .iter()
                        .map(|cell| cell.blocks.iter().map(block_view).collect())
                        .collect(),
                })
                .collect();
        }
        BlockKind::ThematicBreak => {}
        BlockKind::Html { html } => view.text = Some(html.clone()),
    }
    view
}

fn position_view(document: &Document, position: Position) -> WirePosition {
    let offset = content_of(document, position.block)
        .map(|content| utf16_offset(content, position.offset))
        .unwrap_or(0);
    WirePosition {
        block: position.block.0,
        offset,
    }
}

/// Everything the view needs, with offsets converted for JavaScript.
pub fn state_view(name: &str, editor: &Editor) -> WireState {
    let document = editor.document();
    let selection = editor.selection();
    WireState {
        name: name.to_string(),
        revision: editor.revision(),
        selection: WireSelection {
            anchor: position_view(document, selection.anchor),
            head: position_view(document, selection.head),
        },
        blocks: document.blocks().iter().map(block_view).collect(),
        outline: document
            .outline()
            .into_iter()
            .map(|heading| WireHeading {
                id: heading.id.0,
                level: heading.level,
                text: heading.text,
            })
            .collect(),
        active_marks: mark_names(editor.active_marks()),
        word_count: document.plain_text().split_whitespace().count(),
        can_undo: editor.history().current().is_some(),
        can_redo: !editor
            .history()
            .children_newest_first(editor.history().current())
            .is_empty(),
        frontmatter: document
            .metadata
            .frontmatter
            .as_ref()
            .map(|frontmatter| frontmatter.fields.clone()),
    }
}

// --- commands from the frontend ---------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireIncomingPosition {
    pub block: u64,
    /// In UTF-16 code units, as JavaScript counts them.
    pub offset: usize,
}

/// What the UI is allowed to ask for. One flat list, versioned by this file —
/// the frontend never names an operation, a transaction or a block path.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", rename_all = "camelCase")]
pub enum WireCommand {
    SetSelection {
        anchor: WireIncomingPosition,
        head: WireIncomingPosition,
    },
    InsertText {
        text: String,
    },
    Delete {
        #[serde(default)]
        forward: bool,
    },
    SplitBlock,
    MergeBlocks,
    ToggleMark {
        mark: String,
    },
    SetHeadingLevel {
        level: u8,
    },
    SetParagraph,
    SetCodeBlock {
        #[serde(default)]
        language: Option<String>,
    },
    WrapInQuote,
    ToggleList {
        #[serde(default)]
        ordered: bool,
    },
    ToggleTask,
    InsertTable {
        rows: usize,
        columns: usize,
    },
    InsertTableRow {
        #[serde(default)]
        before: bool,
    },
    InsertTableColumn {
        #[serde(default)]
        before: bool,
    },
    SetLink {
        #[serde(default)]
        url: Option<String>,
    },
    InsertImage {
        source: String,
        #[serde(default)]
        alt: String,
    },
    InsertParagraph,
    InsertThematicBreak,
    Undo,
    Redo,
}

impl WireCommand {
    /// Turns a wire command into an engine command, converting offsets against
    /// the document as it is *now* — which is why commands are applied one at a
    /// time and each conversion happens just before its own command runs.
    pub fn into_command(self, document: &Document) -> Result<EditorCommand, String> {
        let position = |incoming: WireIncomingPosition| {
            let block = BlockId(incoming.block);
            let offset = content_of(document, block)
                .map(|content| char_offset(content, incoming.offset))
                .unwrap_or(0);
            Position::new(block, offset)
        };

        Ok(match self {
            WireCommand::SetSelection { anchor, head } => {
                EditorCommand::SetSelection(SelectionRange::new(position(anchor), position(head)))
            }
            WireCommand::InsertText { text } => EditorCommand::InsertText(text),
            WireCommand::Delete { forward } => EditorCommand::Delete(if forward {
                Direction::Forward
            } else {
                Direction::Backward
            }),
            WireCommand::SplitBlock => EditorCommand::SplitBlock,
            WireCommand::MergeBlocks => EditorCommand::MergeBlocks,
            WireCommand::ToggleMark { mark } => EditorCommand::ToggleMark(mark_named(&mark)?),
            WireCommand::SetHeadingLevel { level } => EditorCommand::SetHeadingLevel(level),
            WireCommand::SetParagraph => EditorCommand::SetParagraph,
            WireCommand::SetCodeBlock { language } => EditorCommand::SetCodeBlock { language },
            WireCommand::WrapInQuote => EditorCommand::WrapInQuote,
            WireCommand::ToggleList { ordered } => EditorCommand::ToggleList { ordered },
            WireCommand::ToggleTask => EditorCommand::ToggleTask,
            WireCommand::InsertTable { rows, columns } => {
                EditorCommand::InsertTable { rows, columns }
            }
            WireCommand::InsertTableRow { before } => EditorCommand::InsertTableRow { before },
            WireCommand::InsertTableColumn { before } => {
                EditorCommand::InsertTableColumn { before }
            }
            WireCommand::SetLink { url } => EditorCommand::SetLink { url },
            WireCommand::InsertImage { source, alt } => EditorCommand::InsertImage(ImageNode {
                source,
                alt,
                title: None,
            }),
            WireCommand::InsertParagraph => EditorCommand::InsertParagraph,
            WireCommand::InsertThematicBreak => EditorCommand::InsertThematicBreak,
            WireCommand::Undo => EditorCommand::Undo,
            WireCommand::Redo => EditorCommand::Redo,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_core::{markdown, Link};

    fn content(pieces: Vec<Inline>) -> InlineContent {
        InlineContent::new(pieces)
    }

    #[test]
    fn offsets_convert_both_ways_across_the_bmp_boundary() {
        // "🙂" is one char but two UTF-16 units; "é" and "日" are one of each.
        let content = content(vec![Inline::text("aé日🙂b")]);
        let pairs = [(0, 0), (1, 1), (2, 2), (3, 3), (4, 5), (5, 6)];
        for (chars, utf16) in pairs {
            assert_eq!(
                utf16_offset(&content, chars),
                utf16,
                "chars {chars} -> utf16"
            );
            assert_eq!(
                char_offset(&content, utf16),
                chars,
                "utf16 {utf16} -> chars"
            );
        }
    }

    #[test]
    fn an_offset_inside_a_surrogate_pair_rounds_down() {
        let content = content(vec![Inline::text("🙂x")]);
        // Offset 1 is inside the pair; the character it belongs to starts at 0.
        assert_eq!(char_offset(&content, 1), 0);
        assert_eq!(char_offset(&content, 2), 1);
    }

    #[test]
    fn an_atom_counts_as_one_unit_on_both_sides() {
        let content = content(vec![
            Inline::text("see 🙂 "),
            Inline::Image(ImageNode::new("map.png")),
            Inline::text(" and"),
        ]);
        // "see 🙂 " is 6 chars / 7 units, then the image.
        assert_eq!(utf16_offset(&content, 6), 7);
        assert_eq!(utf16_offset(&content, 7), 8, "the image is one unit");
        assert_eq!(char_offset(&content, 8), 7);
        assert_eq!(char_offset(&content, 11), 10);
    }

    #[test]
    fn offsets_past_the_end_clamp_to_it() {
        let content = content(vec![Inline::text("abc")]);
        assert_eq!(char_offset(&content, 99), 3);
        assert_eq!(utf16_offset(&content, 99), 3);
    }

    #[test]
    fn a_selection_survives_the_round_trip_through_the_wire() {
        let mut editor = Editor::new(markdown::parse("t.md", "🙂 héllo world\n"));
        let block = editor.document().blocks()[0].id;

        // The frontend selects "héllo" using JavaScript offsets.
        let command = WireCommand::SetSelection {
            anchor: WireIncomingPosition {
                block: block.0,
                offset: 3,
            },
            head: WireIncomingPosition {
                block: block.0,
                offset: 8,
            },
        };
        let command = command.into_command(editor.document()).unwrap();
        editor.apply(command).unwrap();
        assert_eq!(editor.selection().single_block_range(), Some((block, 2, 7)));

        editor.apply(EditorCommand::toggle_bold()).unwrap();
        assert_eq!(
            markdown::to_markdown(editor.document()),
            "🙂 **héllo** world\n"
        );

        let state = state_view("t.md", &editor);
        assert_eq!(
            state.selection.anchor.offset, 3,
            "and comes back in JS units"
        );
        assert_eq!(state.selection.head.offset, 8);
    }

    #[test]
    fn the_view_describes_runs_marks_and_links() {
        let editor = Editor::new(markdown::parse(
            "t.md",
            "Some **bold** and [a link](notes.md).\n",
        ));
        let state = state_view("t.md", &editor);
        let runs = &state.blocks[0].runs;

        assert_eq!(runs[0].text, "Some ");
        assert!(runs[0].marks.is_empty());
        assert_eq!(runs[1].text, "bold");
        assert_eq!(runs[1].marks, vec!["bold"]);
        assert_eq!(runs[3].text, "a link");
        assert_eq!(runs[3].href.as_deref(), Some("notes.md"));
        assert_eq!(state.word_count, 5);
    }

    #[test]
    fn nested_blocks_reach_the_view_with_their_ids() {
        let editor = Editor::new(markdown::parse(
            "t.md",
            "> quoted\n\n- [x] done\n\n| a |\n| --- |\n| b |\n",
        ));
        let state = state_view("t.md", &editor);

        assert_eq!(state.blocks[0].kind, "quote");
        assert_eq!(state.blocks[0].children[0].runs[0].text, "quoted");
        assert_eq!(state.blocks[1].items[0].checked, Some(true));
        assert_eq!(state.blocks[2].rows.len(), 2);
        assert_eq!(state.blocks[2].rows[1].cells[0][0].runs[0].text, "b");

        // Every block the caret can be put in is addressable from the view.
        let ids: Vec<u64> = editor
            .document()
            .text_block_ids()
            .into_iter()
            .map(|id| id.0)
            .collect();
        assert!(ids.contains(&state.blocks[0].children[0].id));
        assert!(ids.contains(&state.blocks[2].rows[1].cells[0][0].id));
    }

    #[test]
    fn marks_are_named_both_ways() {
        assert_eq!(mark_named("bold"), Ok(MarkSet::BOLD));
        assert_eq!(mark_named("strikethrough"), Ok(MarkSet::STRIKE));
        assert!(mark_named("rainbow").is_err());
        assert_eq!(
            mark_names(MarkSet::BOLD.with(MarkSet::CODE)),
            vec!["bold", "code"]
        );
    }

    #[test]
    fn the_wire_command_list_parses_from_json() {
        let json = r#"[
            {"command":"insertText","text":"hi"},
            {"command":"delete","forward":true},
            {"command":"toggleMark","mark":"italic"},
            {"command":"setHeadingLevel","level":2},
            {"command":"insertTable","rows":2,"columns":3},
            {"command":"setLink","url":"notes.md"},
            {"command":"undo"}
        ]"#;
        let commands: Vec<WireCommand> = serde_json::from_str(json).unwrap();
        assert_eq!(commands.len(), 7);

        let document = markdown::parse("t.md", "x\n");
        for command in commands {
            command.into_command(&document).unwrap();
        }
    }

    #[test]
    fn an_unlinked_run_keeps_its_title_out_of_the_view() {
        let content = content(vec![Inline::Text {
            text: "x".into(),
            marks: MarkSet::NONE,
            link: Some(Link {
                url: "u".into(),
                title: Some("t".into()),
            }),
        }]);
        let runs = runs_of(&content);
        assert_eq!(runs[0].title.as_deref(), Some("t"));
        assert_eq!(runs[0].href.as_deref(), Some("u"));
    }
}
