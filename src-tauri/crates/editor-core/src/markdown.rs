//! Markdown in, Markdown out — the only module that knows the syntax.
//!
//! ```text
//! Markdown ──parse──▶ Document ──edit──▶ Document ──to_markdown──▶ Markdown
//! ```
//!
//! # Why pulldown-cmark
//!
//! `pulldown-cmark` is already in Memorie's dependency tree (the chunker, the
//! exporter and the PDF writer each walk it), it is CommonMark-correct with
//! the GFM extensions this app needs, and its events carry byte offsets — the
//! hook an incremental reparse would need later.
//!
//! `comrak` was the alternative. It parses to its own AST and can print that
//! AST back to Markdown, which sounds like a shortcut; it isn't, because
//! `Document` is the AST here. Its tree would be a second model to convert
//! through, and its formatter formats *its* nodes, not ours, so the
//! serializer below would still have to exist. One less dependency wins.
//!
//! # What "round-trip" means here
//!
//! Semantics first: parsing, printing and parsing again gives the same
//! `Document`. Byte-for-byte preservation of the source is explicitly *not*
//! promised — `*emphasis*` comes back as `_emphasis_` — with two exceptions
//! that matter to an app whose files live in Git:
//!
//! * front matter is written back verbatim, and
//! * soft line breaks are kept, so somebody's hand-wrapped paragraphs are not
//!   reflowed into one long line on save.

use crate::document::{Document, Frontmatter};
use crate::ids::IdGenerator;
use crate::inline::{ImageNode, Inline, InlineContent, Link, MarkSet};
use crate::node::{Block, BlockKind, ColumnAlignment, ListItem, TableCell, TableNode, TableRow};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Parses a whole file: front matter, then blocks.
pub fn parse(id: impl Into<crate::ids::DocumentId>, text: &str) -> Document {
    let (frontmatter, body) = split_frontmatter(text);
    let mut ids = IdGenerator::default();
    let blocks = parse_blocks(body, &mut ids);
    let mut document = Document::from_blocks(id, blocks);
    document.metadata.frontmatter = frontmatter;
    document
}

/// Parses just the body — no front matter handling — into top-level blocks.
pub fn parse_blocks(body: &str, ids: &mut IdGenerator) -> Vec<Block> {
    // Footnotes and metadata blocks stay off: footnotes have no place in the
    // model yet, so `[^1]` is better left as the text it looks like, and front
    // matter is split off before the parser sees the body.
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut events = Parser::new_ext(body, options).peekable();
    let mut builder = Builder { ids };
    builder.blocks(&mut events, None)
}

/// Prints a whole file: front matter, then blocks.
pub fn to_markdown(document: &Document) -> String {
    let body = blocks_to_markdown(document.blocks(), 0);
    match &document.metadata.frontmatter {
        Some(frontmatter) => format!("{}{body}", frontmatter.to_markdown()),
        None => body,
    }
}

/// Prints blocks only, without front matter.
pub fn blocks_to_markdown(blocks: &[Block], _depth: usize) -> String {
    let mut out = String::new();
    write_blocks(blocks, &mut out, true);
    out
}

// --- front matter -----------------------------------------------------------

/// Splits a leading YAML front matter block off `text`.
///
/// A document may also open with a thematic break, so a block only counts as
/// front matter when it reads like YAML — the same rule the app's JavaScript
/// used, kept so opening a file classifies it the same way it always did.
pub fn split_frontmatter(text: &str) -> (Option<Frontmatter>, &str) {
    let after_open = match text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    {
        Some(rest) => rest,
        None => return (None, text),
    };
    let open_len = text.len() - after_open.len();

    // The closing fence is the next line that is exactly `---`.
    let mut inner_len = 0;
    let mut close_len = 0;
    for line in after_open.split_inclusive('\n') {
        if line.trim_end() == "---" {
            close_len = line.len();
            break;
        }
        inner_len += line.len();
    }
    if close_len == 0 {
        return (None, text);
    }

    let inner = &after_open[..inner_len];
    if !looks_like_yaml(inner) {
        return (None, text);
    }

    // A blank line conventionally follows the block, and belongs to it.
    let mut raw_len = open_len + inner_len + close_len;
    let rest = &text[raw_len..];
    for blank in ["\r\n", "\n"] {
        if rest.starts_with(blank) {
            raw_len += blank.len();
            break;
        }
    }

    let frontmatter = Frontmatter {
        raw: text[..raw_len].to_string(),
        fields: parse_yaml_fields(inner),
    };
    (Some(frontmatter), &text[raw_len..])
}

fn looks_like_yaml(inner: &str) -> bool {
    let mut lines = inner.lines();
    let Some(first) = lines.next() else {
        return false;
    };
    if key_of(first).is_none() {
        return false;
    }
    inner.lines().all(|line| {
        line.is_empty()
            || key_of(line).is_some()
            || line.starts_with(char::is_whitespace)
            || line.starts_with("- ")
            || line.starts_with('#')
    })
}

/// The key of a `key: value` line, if it is one.
fn key_of(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    let mut chars = key.chars();
    let first = chars.next()?;
    let ok = (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    ok.then(|| (key, value.trim()))
}

/// Top-level scalar fields, enough for the metadata this app keeps
/// (`synopsis`, `status`, `label`). Anything else in the block is left to
/// `raw`, which is what gets written back.
fn parse_yaml_fields(inner: &str) -> std::collections::BTreeMap<String, String> {
    let mut fields = std::collections::BTreeMap::new();
    let lines: Vec<&str> = inner.lines().collect();
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at];
        at += 1;
        let Some((key, value)) = key_of(line) else {
            continue;
        };
        // A block scalar's value is the indented lines below it: `|` keeps the
        // line breaks, `>` folds them into spaces.
        if value == "|" || value == ">" {
            let mut body = Vec::new();
            while at < lines.len() && (lines[at].is_empty() || lines[at].starts_with(' ')) {
                body.push(lines[at].trim());
                at += 1;
            }
            let joined = if value == "|" {
                body.join("\n")
            } else {
                body.join(" ")
            };
            fields.insert(key.to_string(), joined.trim().to_string());
        } else if !value.is_empty() {
            fields.insert(key.to_string(), unquote(value));
        }
    }
    fields
}

fn unquote(value: &str) -> String {
    if let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        return inner
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\\\", "\\");
    }
    if let Some(inner) = value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        return inner.replace("''", "'");
    }
    value.to_string()
}

impl Frontmatter {
    /// The block as it should be written, including its trailing blank line.
    /// `raw` wins whenever it is there, so an untouched block is written back
    /// byte for byte and saving never reformats somebody's YAML.
    pub fn to_markdown(&self) -> String {
        if !self.raw.is_empty() {
            return self.raw.clone();
        }
        if self.fields.is_empty() {
            return String::new();
        }
        let mut out = String::from("---\n");
        for (key, value) in &self.fields {
            out.push_str(&format!("{key}: {}\n", quote_yaml(value)));
        }
        out.push_str("---\n\n");
        out
    }
}

/// Written as a JSON string, which is valid YAML, so quotes and newlines survive.
fn quote_yaml(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

// --- parsing ----------------------------------------------------------------

struct Builder<'a> {
    ids: &'a mut IdGenerator,
}

impl Builder<'_> {
    fn block(&mut self, kind: BlockKind) -> Block {
        Block::new(self.ids.next_id(), kind)
    }

    /// Blocks until `until` closes (or the events run out, for the root).
    fn blocks<'e, I>(
        &mut self,
        events: &mut std::iter::Peekable<I>,
        until: Option<TagEnd>,
    ) -> Vec<Block>
    where
        I: Iterator<Item = Event<'e>>,
    {
        let mut blocks = Vec::new();
        while let Some(event) = events.next() {
            match event {
                Event::End(tag) if Some(tag) == until => break,
                Event::Start(tag) => {
                    if let Some(block) = self.start(tag, events) {
                        blocks.push(block);
                    }
                }
                Event::Rule => blocks.push(self.block(BlockKind::ThematicBreak)),
                Event::Html(html) => {
                    // Consecutive HTML events are one block in the source.
                    let mut raw = html.to_string();
                    while let Some(Event::Html(more)) = events.peek() {
                        raw.push_str(more);
                        events.next();
                    }
                    blocks.push(self.block(BlockKind::Html {
                        html: raw.trim_end_matches('\n').to_string(),
                    }));
                }
                // Text outside a paragraph (inside a tight list item) still
                // belongs in one.
                Event::Text(_) | Event::Code(_) | Event::InlineHtml(_) => {
                    let content = InlineContent::from_text(match event {
                        Event::Text(text) | Event::InlineHtml(text) => text.to_string(),
                        Event::Code(code) => code.to_string(),
                        _ => unreachable!(),
                    });
                    blocks.push(self.block(BlockKind::Paragraph { content }));
                }
                _ => {}
            }
        }
        blocks
    }

    /// One block, from its opening tag to its close.
    fn start<'e, I>(&mut self, tag: Tag<'e>, events: &mut std::iter::Peekable<I>) -> Option<Block>
    where
        I: Iterator<Item = Event<'e>>,
    {
        let kind = match tag {
            Tag::Paragraph => {
                let content = self.inline(events, TagEnd::Paragraph);
                BlockKind::Paragraph { content }
            }
            Tag::Heading { level, .. } => {
                let content = self.inline(events, TagEnd::Heading(level));
                BlockKind::Heading {
                    level: heading_level(level),
                    content,
                }
            }
            Tag::BlockQuote(_) => BlockKind::Quote {
                blocks: self.blocks(events, Some(TagEnd::BlockQuote(None))),
            },
            Tag::CodeBlock(kind) => {
                let mut code = String::new();
                for event in events.by_ref() {
                    match event {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(text) => code.push_str(&text),
                        _ => {}
                    }
                }
                let language = match kind {
                    CodeBlockKind::Fenced(info) if !info.trim().is_empty() => {
                        Some(info.trim().to_string())
                    }
                    _ => None,
                };
                BlockKind::CodeBlock {
                    language,
                    code: code.trim_end_matches('\n').to_string(),
                }
            }
            Tag::List(start) => {
                let ordered = start.is_some();
                let mut items = Vec::new();
                let mut tight = true;
                while let Some(event) = events.next() {
                    match event {
                        Event::End(TagEnd::List(_)) => break,
                        Event::Start(Tag::Item) => {
                            let (item, loose) = self.list_item(events);
                            tight &= !loose;
                            items.push(item);
                        }
                        _ => {}
                    }
                }
                BlockKind::List {
                    ordered,
                    start: start.filter(|start| *start != 1),
                    tight,
                    items,
                }
            }
            Tag::Table(alignments) => BlockKind::Table {
                table: self.table(events, alignments),
            },
            Tag::HtmlBlock => {
                let mut raw = String::new();
                for event in events.by_ref() {
                    match event {
                        Event::End(TagEnd::HtmlBlock) => break,
                        Event::Html(html) | Event::Text(html) => raw.push_str(&html),
                        _ => {}
                    }
                }
                BlockKind::Html {
                    html: raw.trim_end_matches('\n').to_string(),
                }
            }
            // Inline tags can't open a block; skipping is the honest response.
            _ => return None,
        };
        Some(self.block(kind))
    }

    /// One list item, and whether it was loose (its text wrapped in a paragraph).
    fn list_item<'e, I>(&mut self, events: &mut std::iter::Peekable<I>) -> (ListItem, bool)
    where
        I: Iterator<Item = Event<'e>>,
    {
        let mut checked = None;
        if let Some(Event::TaskListMarker(ticked)) = events.peek() {
            checked = Some(*ticked);
            events.next();
        }
        let loose = matches!(events.peek(), Some(Event::Start(Tag::Paragraph)));
        let blocks = self.blocks(events, Some(TagEnd::Item));
        (ListItem { checked, blocks }, loose)
    }

    fn table<'e, I>(
        &mut self,
        events: &mut std::iter::Peekable<I>,
        alignments: Vec<Alignment>,
    ) -> TableNode
    where
        I: Iterator<Item = Event<'e>>,
    {
        let mut rows = Vec::new();
        let mut cells = Vec::new();
        while let Some(event) = events.next() {
            match event {
                Event::End(TagEnd::Table) => break,
                Event::Start(Tag::TableCell) => {
                    let content = self.inline(events, TagEnd::TableCell);
                    let block = self.block(BlockKind::Paragraph { content });
                    cells.push(TableCell {
                        blocks: vec![block],
                    });
                }
                Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                    rows.push(TableRow {
                        cells: std::mem::take(&mut cells),
                    });
                }
                _ => {}
            }
        }
        TableNode {
            alignments: alignments.into_iter().map(column_alignment).collect(),
            rows,
        }
    }

    /// Inline content until `until` closes. Marks and links are collected as
    /// they nest and flattened onto each run.
    fn inline<'e, I>(&mut self, events: &mut std::iter::Peekable<I>, until: TagEnd) -> InlineContent
    where
        I: Iterator<Item = Event<'e>>,
    {
        let mut pieces = Vec::new();
        let mut marks = MarkSet::NONE;
        let mut links: Vec<Link> = Vec::new();

        while let Some(event) = events.next() {
            match event {
                Event::End(tag) if tag == until => break,
                Event::Start(Tag::Strong) => marks = marks.with(MarkSet::BOLD),
                Event::End(TagEnd::Strong) => marks = marks.without(MarkSet::BOLD),
                Event::Start(Tag::Emphasis) => marks = marks.with(MarkSet::ITALIC),
                Event::End(TagEnd::Emphasis) => marks = marks.without(MarkSet::ITALIC),
                Event::Start(Tag::Strikethrough) => marks = marks.with(MarkSet::STRIKE),
                Event::End(TagEnd::Strikethrough) => marks = marks.without(MarkSet::STRIKE),
                Event::Start(Tag::Link {
                    dest_url, title, ..
                }) => links.push(Link {
                    url: dest_url.to_string(),
                    title: (!title.is_empty()).then(|| title.to_string()),
                }),
                Event::End(TagEnd::Link) => {
                    links.pop();
                }
                Event::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    // The alt text is the events up to the image's close.
                    let alt = self.inline(events, TagEnd::Image).plain_text();
                    pieces.push(Inline::Image(ImageNode {
                        source: dest_url.to_string(),
                        alt,
                        title: (!title.is_empty()).then(|| title.to_string()),
                    }));
                }
                Event::Text(text) => pieces.push(Inline::Text {
                    text: text.to_string(),
                    marks,
                    link: links.last().cloned(),
                }),
                Event::Code(code) => pieces.push(Inline::Text {
                    text: code.to_string(),
                    marks: marks.with(MarkSet::CODE),
                    link: links.last().cloned(),
                }),
                Event::InlineHtml(html) => pieces.push(Inline::Html(html.to_string())),
                // The author's own line wrapping, kept so saving doesn't
                // reflow the paragraph.
                Event::SoftBreak => pieces.push(Inline::Text {
                    text: "\n".to_string(),
                    marks,
                    link: links.last().cloned(),
                }),
                Event::HardBreak => pieces.push(Inline::HardBreak),
                _ => {}
            }
        }
        InlineContent::new(pieces)
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn column_alignment(alignment: Alignment) -> ColumnAlignment {
    match alignment {
        Alignment::None => ColumnAlignment::None,
        Alignment::Left => ColumnAlignment::Left,
        Alignment::Center => ColumnAlignment::Center,
        Alignment::Right => ColumnAlignment::Right,
    }
}

// --- serializing ------------------------------------------------------------

fn write_blocks(blocks: &[Block], out: &mut String, at_document_start: bool) {
    write_blocks_spaced(blocks, out, at_document_start, true)
}

/// `spaced` is false inside a tight list item, where a paragraph and the list
/// nested under it sit on consecutive lines with no blank line between them.
fn write_blocks_spaced(blocks: &[Block], out: &mut String, at_document_start: bool, spaced: bool) {
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            out.push('\n');
            if spaced {
                out.push('\n');
            }
        }
        write_block(block, out, at_document_start && index == 0);
    }
    if !blocks.is_empty() {
        out.push('\n');
    }
}

fn write_block(block: &Block, out: &mut String, at_document_start: bool) {
    match &block.kind {
        BlockKind::Paragraph { content } => out.push_str(&inline_to_markdown(content)),
        BlockKind::Heading { level, content } => {
            out.push_str(&"#".repeat(*level as usize));
            out.push(' ');
            out.push_str(&inline_at(content, false));
        }
        BlockKind::CodeBlock { language, code } => {
            // A fence has to be longer than any backtick run in the code.
            let longest = code
                .split(|c| c != '`')
                .map(str::len)
                .max()
                .unwrap_or(0)
                .max(2);
            let fence = "`".repeat(longest + 1);
            out.push_str(&fence);
            out.push_str(language.as_deref().unwrap_or(""));
            out.push('\n');
            out.push_str(code);
            out.push('\n');
            out.push_str(&fence);
        }
        BlockKind::Quote { blocks } => {
            let mut inner = String::new();
            write_blocks(blocks, &mut inner, false);
            out.push_str(&prefix_lines(inner.trim_end_matches('\n'), "> ", "> "));
        }
        BlockKind::List {
            ordered,
            start,
            tight,
            items,
        } => {
            write_list(*ordered, *start, *tight, items, out);
        }
        BlockKind::Image { image } => out.push_str(&image_to_markdown(image)),
        BlockKind::Table { table } => write_table(table, out),
        // `---` at the very start of a document would be read back as front
        // matter, so there it is spelled `***` instead.
        BlockKind::ThematicBreak => out.push_str(if at_document_start { "***" } else { "---" }),
        BlockKind::Html { html } => out.push_str(html),
    }
}

fn write_list(
    ordered: bool,
    start: Option<u64>,
    tight: bool,
    items: &[ListItem],
    out: &mut String,
) {
    let mut number = start.unwrap_or(1);
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
            if !tight {
                out.push('\n');
            }
        }
        let marker = if ordered {
            let marker = format!("{number}. ");
            number += 1;
            marker
        } else {
            "- ".to_string()
        };
        let marker = match item.checked {
            Some(true) => format!("{marker}[x] "),
            Some(false) => format!("{marker}[ ] "),
            None => marker,
        };

        let mut inner = String::new();
        write_blocks_spaced(&item.blocks, &mut inner, false, !tight);
        let indent = " ".repeat(marker.chars().count());
        out.push_str(&prefix_lines(
            inner.trim_end_matches('\n'),
            &marker,
            &indent,
        ));
    }
}

fn write_table(table: &TableNode, out: &mut String) {
    let columns = table
        .rows
        .iter()
        .map(|row| row.cells.len())
        .max()
        .unwrap_or(0);
    let cell_text = |cell: &TableCell| {
        cell.blocks
            .iter()
            .filter_map(Block::content)
            .map(|content| inline_at(content, false))
            .collect::<Vec<_>>()
            .join(" ")
            .replace('|', "\\|")
            .replace('\n', " ")
    };

    for (index, row) in table.rows.iter().enumerate() {
        let mut cells: Vec<String> = row.cells.iter().map(cell_text).collect();
        cells.resize(columns, String::new());
        out.push_str(&format!("| {} |\n", cells.join(" | ")));

        if index == 0 {
            let rules: Vec<&str> = (0..columns)
                .map(|column| match table.alignments.get(column) {
                    Some(ColumnAlignment::Left) => ":---",
                    Some(ColumnAlignment::Center) => ":---:",
                    Some(ColumnAlignment::Right) => "---:",
                    _ => "---",
                })
                .collect();
            out.push_str(&format!("| {} |\n", rules.join(" | ")));
        }
    }
    while out.ends_with('\n') {
        out.pop();
    }
}

/// Puts `first` in front of the first line and `rest` in front of the others,
/// which is how both quotes and list items indent their contents.
fn prefix_lines(text: &str, first: &str, rest: &str) -> String {
    text.split('\n')
        .enumerate()
        .map(|(index, line)| {
            let prefix = if index == 0 { first } else { rest };
            if line.is_empty() && index > 0 {
                prefix.trim_end().to_string()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn image_to_markdown(image: &ImageNode) -> String {
    let title = image
        .title
        .as_ref()
        .map(|title| format!(" \"{}\"", title.replace('"', "\\\"")))
        .unwrap_or_default();
    format!(
        "![{}]({}{title})",
        escape_text(&image.alt, false),
        url(&image.source)
    )
}

/// A URL needs angle brackets when it holds anything that would end the link
/// early.
fn url(source: &str) -> String {
    if source.contains([' ', '(', ')']) {
        format!("<{source}>")
    } else {
        source.to_string()
    }
}

/// Prints inline content that starts at the beginning of a line, the usual
/// case (a paragraph, or a list item's first paragraph).
pub fn inline_to_markdown(content: &InlineContent) -> String {
    inline_at(content, true)
}

/// Prints inline content, saying whether it begins at the start of a line —
/// which is what decides whether a leading `#`, `-` or `1.` has to be escaped.
pub fn inline_at(content: &InlineContent, line_start: bool) -> String {
    let mut out = String::new();
    let mut at_line_start = line_start;
    emit_runs(
        content.pieces(),
        MarkSet::NONE,
        None,
        &mut out,
        &mut at_line_start,
    );
    out
}

/// Marks are written outermost first, so nested markup comes out in a stable
/// order; code, inside which nothing is marked up, is innermost.
const MARK_ORDER: [(MarkSet, &str); 3] = [
    (MarkSet::STRIKE, "~~"),
    (MarkSet::BOLD, "**"),
    (MarkSet::ITALIC, "_"),
];

/// Prints runs, wrapping each maximal group that shares a link or a mark in one
/// pair of markers.
///
/// Grouping is what keeps `**bold _and italic_**` from coming back as
/// `**bold** **_and italic_**`: the space between those two runs is bold in the
/// model, and only a shared wrapper can say so.
fn emit_runs(
    runs: &[Inline],
    applied: MarkSet,
    link: Option<&Link>,
    out: &mut String,
    at_line_start: &mut bool,
) {
    let mut at = 0;
    while at < runs.len() {
        let (text, marks, run_link) = match &runs[at] {
            Inline::Text { text, marks, link } => (text, *marks, link.as_ref()),
            // Atoms carry no marks of their own.
            Inline::Image(image) => {
                out.push_str(&image_to_markdown(image));
                *at_line_start = false;
                at += 1;
                continue;
            }
            Inline::Html(html) => {
                out.push_str(html);
                *at_line_start = html.ends_with('\n');
                at += 1;
                continue;
            }
            Inline::HardBreak => {
                out.push_str("\\\n");
                *at_line_start = true;
                at += 1;
                continue;
            }
        };

        // A link wraps everything inside it, so it is grouped first.
        if run_link != link {
            match run_link {
                Some(target) => {
                    let end = group_end(runs, at, |piece| link_of(piece) == Some(target));
                    out.push('[');
                    *at_line_start = false;
                    emit_runs(&runs[at..end], applied, Some(target), out, at_line_start);
                    out.push_str(&format!(
                        "]({}{})",
                        url(&target.url),
                        title_suffix(&target.title)
                    ));
                    *at_line_start = false;
                    at = end;
                    continue;
                }
                // This run is outside the link being printed; it belongs to
                // the level above, which the caller is still walking.
                None => return,
            }
        }

        let remaining = marks.without(applied);
        if remaining.is_empty() {
            out.push_str(&escape_text(text, *at_line_start));
            *at_line_start = text.ends_with('\n');
            at += 1;
            continue;
        }

        // Code is innermost and holds raw text, so its group is written in one
        // go rather than recursed into.
        if remaining == MarkSet::CODE {
            let end = group_end(runs, at, |piece| match piece {
                Inline::Text {
                    marks,
                    link: piece_link,
                    ..
                } => marks.without(applied) == MarkSet::CODE && piece_link.as_ref() == link,
                _ => false,
            });
            let joined: String = runs[at..end].iter().filter_map(text_of).collect();
            out.push_str(&code_span(&joined));
            *at_line_start = false;
            at = end;
            continue;
        }

        let (mark, marker) = MARK_ORDER
            .iter()
            .copied()
            .find(|(mark, _)| remaining.contains(*mark))
            .expect("a mark set that isn't empty and isn't code alone holds one of these");
        let end = group_end(runs, at, |piece| match piece {
            Inline::Text {
                marks,
                link: piece_link,
                ..
            } => marks.contains(mark) && piece_link.as_ref() == link,
            _ => false,
        });

        // Emphasis markers can't sit against whitespace — `**a **` is not
        // strong — so whitespace at either edge of the group moves outside.
        let (lead, group, tail) = peel_whitespace(runs[at..end].to_vec());
        if group.is_empty() {
            let whitespace = format!("{lead}{tail}");
            out.push_str(&escape_text(&whitespace, *at_line_start));
            *at_line_start = whitespace.ends_with('\n');
            at = end;
            continue;
        }

        if !lead.is_empty() {
            out.push_str(&escape_text(&lead, *at_line_start));
            *at_line_start = lead.ends_with('\n');
        }
        out.push_str(marker);
        *at_line_start = false;
        emit_runs(&group, applied.with(mark), link, out, at_line_start);
        out.push_str(marker);
        *at_line_start = false;
        if !tail.is_empty() {
            out.push_str(&escape_text(&tail, false));
            *at_line_start = tail.ends_with('\n');
        }
        at = end;
    }
}

/// Where the run of pieces satisfying `keep` that starts at `from` ends.
fn group_end(runs: &[Inline], from: usize, keep: impl Fn(&Inline) -> bool) -> usize {
    let mut end = from + 1;
    while end < runs.len() && keep(&runs[end]) {
        end += 1;
    }
    end
}

fn link_of(piece: &Inline) -> Option<&Link> {
    match piece {
        Inline::Text { link, .. } => link.as_ref(),
        _ => None,
    }
}

fn text_of(piece: &Inline) -> Option<&str> {
    match piece {
        Inline::Text { text, .. } => Some(text.as_str()),
        _ => None,
    }
}

fn title_suffix(title: &Option<String>) -> String {
    title
        .as_ref()
        .map(|title| format!(" \"{}\"", title.replace('"', "\\\"")))
        .unwrap_or_default()
}

/// Splits whitespace off both ends of a group of runs, so it can be printed
/// outside the emphasis markers.
fn peel_whitespace(mut group: Vec<Inline>) -> (String, Vec<Inline>, String) {
    let is_space = |c: char| c == ' ' || c == '\n';
    let mut lead = String::new();
    let mut tail = String::new();

    if let Some(Inline::Text { text, .. }) = group.first_mut() {
        let trimmed = text.trim_start_matches(is_space).to_string();
        lead = text[..text.len() - trimmed.len()].to_string();
        *text = trimmed;
    }
    if group.first().map(Inline::is_empty).unwrap_or(false) {
        group.remove(0);
    }
    if let Some(Inline::Text { text, .. }) = group.last_mut() {
        let trimmed = text.trim_end_matches(is_space).to_string();
        tail = text[trimmed.len()..].to_string();
        *text = trimmed;
    }
    if group.last().map(Inline::is_empty).unwrap_or(false) {
        group.pop();
    }
    (lead, group, tail)
}

/// A code span, with a fence longer than any backtick run inside it and
/// padding when the content would otherwise touch the fence.
fn code_span(code: &str) -> String {
    let longest = code.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    let pad = if code.starts_with('`') || code.ends_with('`') || code.starts_with(' ') {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{code}{pad}{fence}")
}

/// Escapes what would otherwise be read back as markup. Characters that only
/// mean something at the start of a line are escaped only there, so ordinary
/// prose stays readable.
fn escape_text(text: &str, mut at_line_start: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\n' => {
                out.push('\n');
                at_line_start = true;
                continue;
            }
            '\\' | '*' | '_' | '`' | '[' | ']' | '<' => {
                out.push('\\');
                out.push(c);
            }
            '#' | '>' | '+' if at_line_start => {
                out.push('\\');
                out.push(c);
            }
            '-' if at_line_start => {
                out.push('\\');
                out.push('-');
            }
            '=' if at_line_start => {
                out.push('\\');
                out.push('=');
            }
            // `1.` or `1)` at the start of a line starts a list.
            c if c.is_ascii_digit() && at_line_start => {
                out.push(c);
                let mut digits = String::new();
                while let Some(next) = chars.peek().copied().filter(char::is_ascii_digit) {
                    digits.push(next);
                    chars.next();
                }
                out.push_str(&digits);
                if matches!(chars.peek(), Some('.') | Some(')')) {
                    out.push('\\');
                }
            }
            c => out.push(c),
        }
        at_line_start = false;
    }
    out
}

/// A document's plain text and Markdown, for the systems that consume
/// documents rather than edit them: the indexer, search, export, the AI
/// prompts. Anything that only needs to *read* a document should take this
/// rather than a `&Document`, so a future source (a buffer, a remote document)
/// can stand in.
pub trait DocumentSource {
    fn document(&self) -> &Document;

    fn plain_text(&self) -> String {
        self.document().plain_text()
    }

    fn markdown(&self) -> String {
        to_markdown(self.document())
    }
}

impl DocumentSource for Document {
    fn document(&self) -> &Document {
        self
    }
}

/// Convenience: parse, so callers don't have to name the module.
impl Document {
    pub fn from_markdown(id: impl Into<crate::ids::DocumentId>, text: &str) -> Self {
        parse(id, text)
    }

    pub fn to_markdown(&self) -> String {
        to_markdown(self)
    }
}
