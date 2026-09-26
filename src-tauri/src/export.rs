//! Compiles writings into a single manuscript: DOCX, PDF, EPUB, standalone
//! HTML, or one combined Markdown file.
//!
//! The frontend decides what goes in and in which order (it already knows the
//! binder tree as the writer sees it) and sends a flat list of items: folder
//! headings and documents. This module reads the documents and renders them.

use docx_rs::{
    AlignmentType, BreakType, Docx, FieldCharType, Header, InstrPAGE, InstrText, LineSpacing,
    PageMargin, Paragraph, Run, RunFonts, SpecialIndentType, Style, StyleType, TableOfContents,
    TableOfContentsItem,
};
use epub_builder::{EpubBuilder, EpubContent, ZipLibrary};
use pulldown_cmark::{html, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

/// Links between writings (`[Scene](#writing/Novel/Scene.md)`) mean nothing
/// outside the app, so exports keep their text and drop the link.
const DOC_REF_PREFIX: &str = "#writing/";

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Docx,
    Epub,
    Html,
    Markdown,
    Pdf,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            ExportFormat::Docx => "docx",
            ExportFormat::Epub => "epub",
            ExportFormat::Html => "html",
            ExportFormat::Markdown => "md",
            ExportFormat::Pdf => "pdf",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ExportItem {
    /// A folder (part, chapter) title. Level 1 is the outermost.
    Heading { level: u8, text: String },
    /// A writing, by content-relative name. `title` is shown above it when set.
    Document { name: String, title: Option<String> },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub title: String,
    #[serde(default)]
    pub author: String,
    pub items: Vec<ExportItem>,
    /// Put between consecutive writings that have no heading between them,
    /// e.g. "* * *". Empty for nothing.
    #[serde(default)]
    pub scene_separator: String,
    /// DOCX only: standard manuscript format (12pt Times New Roman, double
    /// spaced, indented paragraphs, 1" margins).
    #[serde(default)]
    pub manuscript_format: bool,
    /// Start every top-level heading on a new page (DOCX, HTML/PDF) or file (EPUB).
    #[serde(default = "default_true")]
    pub chapter_page_breaks: bool,
    /// PDF only.
    #[serde(default)]
    pub page_size: crate::pdf::PageSize,
    /// ISO 639 language of the text (e.g. "en", "de"), for hyphenation and
    /// quote style in PDFs and the EPUB's language.
    #[serde(default)]
    pub lang: Option<String>,
    /// Replaces each chapter's heading, e.g. "Chapter {n}: {title}". Tokens:
    /// `{n}` (3), `{word}` (Three), `{roman}` (III), `{title}` (the folder or
    /// writing name). Empty keeps headings as they are. See [`chapter_heading`].
    #[serde(default)]
    pub chapter_heading: String,
    /// Writings whose scene status matches one of these (ignoring case) are
    /// left out, e.g. "To Do". Folders left empty are dropped too.
    #[serde(default)]
    pub exclude_statuses: Vec<String>,
    /// A contents page after the title page listing parts and chapters
    /// (DOCX, PDF, EPUB).
    #[serde(default)]
    pub table_of_contents: bool,
    /// "Surname / TITLE / page" at the top of every page but the title page,
    /// as agents and editors expect in submissions (DOCX, PDF).
    #[serde(default)]
    pub running_head: bool,
    /// Pages between the title page and the contents (copyright, dedication…).
    #[serde(default)]
    pub front_matter: Vec<MatterItem>,
    /// Pages after the last chapter (acknowledgments, about the author…).
    #[serde(default)]
    pub back_matter: Vec<MatterItem>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatterKind {
    Copyright,
    Dedication,
    Epigraph,
    Foreword,
    Afterword,
    Acknowledgments,
    AboutTheAuthor,
}

/// How a page of front or back matter is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatterStyle {
    /// Small print, e.g. the copyright page.
    Small,
    /// Centered and set lower on the page, e.g. a dedication.
    Centered,
    /// A heading over ordinary text.
    Headed(&'static str),
}

impl MatterKind {
    fn style(self) -> MatterStyle {
        match self {
            MatterKind::Copyright => MatterStyle::Small,
            MatterKind::Dedication | MatterKind::Epigraph => MatterStyle::Centered,
            MatterKind::Foreword => MatterStyle::Headed("Foreword"),
            MatterKind::Afterword => MatterStyle::Headed("Afterword"),
            MatterKind::Acknowledgments => MatterStyle::Headed("Acknowledgments"),
            MatterKind::AboutTheAuthor => MatterStyle::Headed("About the Author"),
        }
    }

    fn id(self) -> &'static str {
        match self {
            MatterKind::Copyright => "copyright",
            MatterKind::Dedication => "dedication",
            MatterKind::Epigraph => "epigraph",
            MatterKind::Foreword => "foreword",
            MatterKind::Afterword => "afterword",
            MatterKind::Acknowledgments => "acknowledgments",
            MatterKind::AboutTheAuthor => "about-the-author",
        }
    }

    fn epub_reftype(self) -> epub_builder::ReferenceType {
        use epub_builder::ReferenceType as R;
        match self {
            MatterKind::Copyright => R::Copyright,
            MatterKind::Dedication => R::Dedication,
            MatterKind::Epigraph => R::Epigraph,
            MatterKind::Foreword => R::Foreword,
            MatterKind::Afterword => R::Text,
            MatterKind::Acknowledgments => R::Acknowledgements,
            MatterKind::AboutTheAuthor => R::Colophon,
        }
    }
}

/// One page of front or back matter: Markdown typed in the export dialog, or
/// a writing (`name`, content-relative) whose text is used instead.
#[derive(Debug, Clone, Deserialize)]
pub struct MatterItem {
    pub kind: MatterKind,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// A matter page ready to render; its Markdown has tokens filled in.
#[derive(Debug, Clone)]
struct Matter {
    kind: MatterKind,
    markdown: String,
}

impl Matter {
    /// Its own headings sit below the page heading (or at level 2 without one).
    fn body(&self) -> String {
        demote_headings(self.markdown.trim(), 1)
    }
}

impl ExportRequest {
    /// The running head before the page number, e.g. "Writer / MY NOVEL",
    /// or `None` when it is turned off.
    fn running_head_text(&self) -> Option<String> {
        if !self.running_head {
            return None;
        }
        let surname = self.author.split_whitespace().last().unwrap_or("");
        let title = self.title.trim().to_uppercase();
        Some([surname, title.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" / "))
    }

    /// The language code if it looks valid, else English.
    fn language(&self) -> &str {
        self.lang
            .as_deref()
            .map(str::trim)
            .filter(|l| (2..=3).contains(&l.len()) && l.chars().all(|c| c.is_ascii_lowercase()))
            .unwrap_or("en")
    }
}

fn default_true() -> bool {
    true
}

/// A document with its Markdown already loaded, in manuscript order.
#[derive(Clone)]
enum Part {
    Heading { level: u8, text: String },
    /// `label` is the file name without extension, for tables of contents
    /// when the writing's title isn't printed.
    Document { title: Option<String>, label: String, markdown: String, status: Option<String> },
}

/// Rejects names that could escape the content directory.
fn safe_join(content_dir: &Path, name: &str) -> Result<PathBuf, String> {
    let relative = Path::new(name);
    let escapes = relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)));
    if escapes {
        return Err(format!("Invalid writing name: {name}"));
    }
    Ok(content_dir.join(relative))
}

/// Drops the scene metadata block (synopsis, status…) that the app keeps at
/// the top of a writing. Same rule as the editor: a `---` block whose first
/// line is `key: value`.
pub fn strip_front_matter(markdown: &str) -> &str {
    let Some(rest) = markdown.strip_prefix("---\n").or_else(|| markdown.strip_prefix("---\r\n")) else {
        return markdown;
    };
    let is_key = |line: &str| {
        line.split_once(':').is_some_and(|(key, _)| {
            key.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
    };
    if !is_key(rest.lines().next().unwrap_or("")) {
        return markdown;
    }
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        offset += line.len();
        let content = line.trim_end_matches(['\r', '\n']);
        if content.trim_end() == "---" {
            return rest[offset..].trim_start_matches(['\r', '\n']);
        }
        // Same rule as the editor: every line must read like YAML.
        let yaml_like = content.is_empty()
            || is_key(content)
            || content.starts_with([' ', '\t', '#'])
            || content.starts_with("- ");
        if !yaml_like {
            return markdown;
        }
    }
    markdown
}

fn load_parts(content_dir: &Path, items: &[ExportItem]) -> Result<Vec<Part>, String> {
    items
        .iter()
        .map(|item| match item {
            ExportItem::Heading { level, text } => Ok(Part::Heading {
                level: (*level).clamp(1, 6),
                text: text.clone(),
            }),
            ExportItem::Document { name, title } => {
                let path = safe_join(content_dir, name)?;
                let markdown = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Could not read \"{name}\": {e}"))?;
                let status = front_matter_value(&markdown, "status");
                let markdown = strip_front_matter(&markdown).to_string();
                let label = Path::new(name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Ok(Part::Document { title: title.clone(), label, markdown, status })
            }
        })
        .collect()
}

/// Front or back matter with `{year}`, `{author}` and `{title}` filled in.
/// Pages that come out empty are skipped.
fn load_matter(request: &ExportRequest, content_dir: &Path, items: &[MatterItem]) -> Result<Vec<Matter>, String> {
    let year = chrono::Local::now().format("%Y").to_string();
    let mut out = Vec::new();
    for item in items {
        let text = match item.name.as_deref().filter(|n| !n.is_empty()) {
            Some(name) => {
                let path = safe_join(content_dir, name)?;
                let markdown = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Could not read \"{name}\": {e}"))?;
                strip_front_matter(&markdown).to_string()
            }
            None => item.text.clone(),
        };
        let markdown = text
            .replace("{year}", &year)
            .replace("{author}", request.author.trim())
            .replace("{title}", request.title.trim());
        if !markdown.trim().is_empty() {
            out.push(Matter { kind: item.kind, markdown });
        }
    }
    Ok(out)
}

/// Everything a render needs, loaded.
struct Book {
    parts: Vec<Part>,
    front: Vec<Matter>,
    back: Vec<Matter>,
}

fn load_book(request: &ExportRequest, content_dir: &Path) -> Result<Book, String> {
    Ok(Book {
        parts: load_manuscript(request, content_dir)?,
        front: load_matter(request, content_dir, &request.front_matter)?,
        back: load_matter(request, content_dir, &request.back_matter)?,
    })
}

/// The parts to render: loaded, filtered by status, with chapter headings applied.
fn load_manuscript(request: &ExportRequest, content_dir: &Path) -> Result<Vec<Part>, String> {
    let mut parts = load_parts(content_dir, &request.items)?;
    let excluded: Vec<String> = request
        .exclude_statuses
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if !excluded.is_empty() {
        parts.retain(|part| match part {
            Part::Document { status: Some(status), .. } => !excluded.contains(&status.trim().to_lowercase()),
            _ => true,
        });
        parts = drop_empty_headings(parts);
        if !parts.iter().any(|p| matches!(p, Part::Document { .. })) {
            return Err("Every writing was left out by the status filter.".into());
        }
    }
    apply_chapter_headings(&request.chapter_heading, &mut parts);
    Ok(parts)
}

/// The value of `key` in a writing's metadata block, unquoted, if set.
fn front_matter_value(markdown: &str, key: &str) -> Option<String> {
    let rest = strip_front_matter(markdown);
    if rest.len() == markdown.len() {
        return None;
    }
    let block = &markdown[..markdown.len() - rest.len()];
    let raw = block.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        (k == key).then_some(v.trim())
    })?;
    let value = if raw.starts_with('"') {
        serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.trim_matches('"').to_string())
    } else if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
        raw[1..raw.len() - 1].replace("''", "'")
    } else {
        raw.to_string()
    };
    Some(value).filter(|v| !v.trim().is_empty())
}

/// Index just past the section a heading at `i` opens: up to the next heading
/// at the same level or above.
fn section_end(parts: &[Part], i: usize, level: u8) -> usize {
    parts[i + 1..]
        .iter()
        .position(|p| matches!(p, Part::Heading { level: l, .. } if *l <= level))
        .map_or(parts.len(), |offset| i + 1 + offset)
}

/// Removes headings whose section has no writings left in it.
fn drop_empty_headings(parts: Vec<Part>) -> Vec<Part> {
    let keep: Vec<bool> = (0..parts.len())
        .map(|i| match &parts[i] {
            Part::Heading { level, .. } => parts[i + 1..section_end(&parts, i, *level)]
                .iter()
                .any(|p| matches!(p, Part::Document { .. })),
            Part::Document { .. } => true,
        })
        .collect();
    parts.into_iter().zip(keep).filter_map(|(p, k)| k.then_some(p)).collect()
}

/// Rewrites chapter headings from `template`. Chapters are the innermost
/// folders (those with no folders inside), numbered through the whole book,
/// so parts stay as they are. With no folders at all, every writing is a chapter.
fn apply_chapter_headings(template: &str, parts: &mut [Part]) {
    if template.trim().is_empty() {
        return;
    }
    let has_headings = parts.iter().any(|p| matches!(p, Part::Heading { .. }));
    let mut n = 0;
    for i in 0..parts.len() {
        let is_chapter = match &parts[i] {
            Part::Heading { level, .. } => !parts[i + 1..section_end(parts, i, *level)]
                .iter()
                .any(|p| matches!(p, Part::Heading { .. })),
            Part::Document { .. } => !has_headings,
        };
        if !is_chapter {
            continue;
        }
        n += 1;
        match &mut parts[i] {
            Part::Heading { text, .. } => *text = chapter_heading(template, n, text),
            Part::Document { title, label, .. } => {
                let name = title.as_deref().filter(|t| !t.trim().is_empty()).unwrap_or(label);
                *title = Some(chapter_heading(template, n, name));
            }
        }
    }
}

/// Entries for a table of contents as (level, text): the folder headings, or
/// every writing's title when there are no folders. Levels are the heading
/// levels the entries get in the compiled body (with no book title above).
fn toc_entries(parts: &[Part]) -> Vec<(usize, String)> {
    let headings: Vec<(usize, String)> = parts
        .iter()
        .filter_map(|p| match p {
            Part::Heading { level, text } => Some((*level as usize, text.clone())),
            Part::Document { .. } => None,
        })
        .collect();
    if !headings.is_empty() {
        return headings;
    }
    parts
        .iter()
        .filter_map(|p| match p {
            Part::Document { title: Some(title), .. } if !title.trim().is_empty() => Some((1, title.trim().to_string())),
            _ => None,
        })
        .collect()
}

/// Fills in a chapter heading template. Falls back to `name` if the result is blank.
fn chapter_heading(template: &str, n: usize, name: &str) -> String {
    let heading = template
        .replace("{n}", &n.to_string())
        .replace("{word}", &number_word(n))
        .replace("{roman}", &roman(n))
        .replace("{title}", name.trim());
    let heading = heading.trim();
    if heading.is_empty() { name.to_string() } else { heading.to_string() }
}

/// "One" to "Ninety-Nine", then digits.
fn number_word(n: usize) -> String {
    const ONES: [&str; 20] = [
        "Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Eleven",
        "Twelve", "Thirteen", "Fourteen", "Fifteen", "Sixteen", "Seventeen", "Eighteen", "Nineteen",
    ];
    const TENS: [&str; 10] =
        ["", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety"];
    match n {
        0..=19 => ONES[n].to_string(),
        20..=99 if n % 10 == 0 => TENS[n / 10].to_string(),
        20..=99 => format!("{}-{}", TENS[n / 10], ONES[n % 10]),
        _ => n.to_string(),
    }
}

/// Roman numerals up to 3999, then digits.
fn roman(mut n: usize) -> String {
    if n == 0 || n > 3999 {
        return n.to_string();
    }
    const NUMERALS: [(usize, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut out = String::new();
    for (value, numeral) in NUMERALS {
        while n >= value {
            out.push_str(numeral);
            n -= value;
        }
    }
    out
}

/// Markdown for a whole compile: headings become Markdown headings, writings
/// get their own titles one level below the heading they sit under.
fn compile_markdown(request: &ExportRequest, parts: &[Part]) -> String {
    let mut out = String::new();
    if !request.title.trim().is_empty() {
        out.push_str(&format!("# {}\n\n", escape_inline_markdown(request.title.trim())));
        if !request.author.trim().is_empty() {
            out.push_str(&format!("*{}*\n\n", escape_inline_markdown(request.author.trim())));
        }
    }
    // Everything shifts down one level under the book title.
    let shift = if request.title.trim().is_empty() { 0 } else { 1 };
    let mut current_level = shift;
    let mut previous_was_document = false;

    for part in parts {
        match part {
            Part::Heading { level, text } => {
                let level = (*level as usize + shift).min(6);
                out.push_str(&format!("{} {}\n\n", "#".repeat(level), escape_inline_markdown(text)));
                current_level = level;
                previous_was_document = false;
            }
            Part::Document { title, markdown, .. } => {
                if previous_was_document && !request.scene_separator.trim().is_empty() {
                    out.push_str(&format!("{}\n\n", escape_separator(&request.scene_separator)));
                }
                let mut inner_level = current_level;
                if let Some(title) = title.as_deref().filter(|t| !t.trim().is_empty()) {
                    inner_level = (current_level + 1).min(6);
                    out.push_str(&format!("{} {}\n\n", "#".repeat(inner_level), escape_inline_markdown(title.trim())));
                }
                // A writing's own headings sit below the folder and title
                // headings, so a "# Heading" in a scene never becomes a chapter.
                out.push_str(demote_headings(markdown.trim(), inner_level).trim());
                out.push_str("\n\n");
                previous_was_document = true;
            }
        }
    }
    out
}

/// The whole book as one Markdown file, matter included.
fn compile_book_markdown(request: &ExportRequest, book: &Book) -> String {
    let full = compile_markdown(request, &book.parts);
    // compile_markdown starts with the title block; matter goes right after it.
    let title_block = compile_markdown(request, &[]);
    let body = full.strip_prefix(title_block.as_str()).unwrap_or(&full);
    let level = if request.title.trim().is_empty() { 1 } else { 2 };
    let matter = |pages: &[Matter]| -> String {
        pages
            .iter()
            .map(|m| {
                let body = demote_headings(m.markdown.trim(), level);
                match m.kind.style() {
                    MatterStyle::Headed(heading) => format!("{} {heading}\n\n{}\n\n", "#".repeat(level), body.trim()),
                    _ => format!("{}\n\n", body.trim()),
                }
            })
            .collect()
    };
    format!("{title_block}{}{body}{}", matter(&book.front), matter(&book.back))
        .trim_end()
        .to_string()
        + "\n"
}

/// Folder names and titles are plain text; keep Markdown from reading them as syntax.
fn escape_inline_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '[' | ']' | '<' | '>' | '#' | '`' | '~' | '|') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Pushes ATX headings (`# ...`) down by `by` levels, capped at 6, leaving
/// fenced code blocks alone.
fn demote_headings(markdown: &str, by: usize) -> String {
    if by == 0 {
        return markdown.to_string();
    }
    let mut out = String::with_capacity(markdown.len() + 16);
    let mut fence: Option<&str> = None;
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if indent <= 3 {
            for marker in ["```", "~~~"] {
                if trimmed.starts_with(marker) {
                    fence = match fence {
                        Some(open) if open == marker => None,
                        None => Some(marker),
                        other => other,
                    };
                }
            }
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        let is_heading = fence.is_none()
            && indent <= 3
            && (1..=6).contains(&hashes)
            && trimmed[hashes..].starts_with([' ', '\t', '\n', '\r']);
        if is_heading {
            out.push_str(&"#".repeat((hashes + by).min(6)));
            out.push_str(&trimmed[hashes..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// "* * *" on its own line is a thematic break in Markdown; keep it literal and centred.
fn escape_separator(separator: &str) -> String {
    let trimmed = separator.trim();
    format!("<p class=\"separator\">{}</p>", html_escape(trimmed))
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn markdown_options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES
}

/// Events with app-only syntax removed: writing links keep their text, raw
/// HTML is shown as text (EPUB needs valid XHTML), except our own separator.
fn clean_events(markdown: &str) -> Vec<Event<'_>> {
    let mut skip_link_end = Vec::new();
    let mut events = Vec::new();
    for event in Parser::new_ext(markdown, markdown_options()) {
        match event {
            Event::Start(Tag::Link { ref dest_url, .. }) => {
                let internal = dest_url.starts_with(DOC_REF_PREFIX);
                skip_link_end.push(internal);
                if !internal {
                    events.push(event);
                }
            }
            Event::End(TagEnd::Link) => {
                if !skip_link_end.pop().unwrap_or(false) {
                    events.push(event);
                }
            }
            // Images point at files we don't bundle; keep the alt text only.
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => {}
            Event::Html(ref raw) | Event::InlineHtml(ref raw) => {
                if raw.trim_start().starts_with("<p class=\"separator\">") {
                    events.push(event);
                } else {
                    events.push(Event::Text(CowStr::from(raw.to_string())));
                }
            }
            other => events.push(other),
        }
    }
    events
}

fn markdown_to_html(markdown: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, clean_events(markdown).into_iter());
    out
}

const BOOK_CSS: &str = r#"
body { font-family: Georgia, "Times New Roman", serif; line-height: 1.6; max-width: 38em; margin: 0 auto; padding: 2em 1.5em; }
h1, h2, h3, h4 { line-height: 1.25; font-weight: normal; }
h1.book-title { text-align: center; margin-top: 30vh; font-size: 2.4em; }
p.book-author { text-align: center; font-style: italic; }
p { margin: 0 0 0.9em; }
p.separator { text-align: center; margin: 1.5em 0; letter-spacing: 0.3em; }
blockquote { margin: 1em 2em; font-style: italic; }
pre, code { font-family: Menlo, Consolas, monospace; font-size: 0.85em; }
table { border-collapse: collapse; }
td, th { border: 1px solid #999; padding: 0.25em 0.5em; }
section.matter { margin: 3em 0; }
.matter-small { font-size: 0.85em; }
.matter-centered { text-align: center; font-style: italic; margin-top: 25vh; }
h2.matter-heading { text-align: center; }
"#;

const PRINT_CSS: &str = r#"
@page { margin: 2.5cm; }
@media print {
  body { max-width: none; padding: 0; }
  .page-break { break-before: page; }
  h1, h2, h3 { break-after: avoid; }
}
"#;

/// One standalone HTML page for the whole manuscript. Also what gets printed to PDF.
fn render_html(request: &ExportRequest, book: &Book) -> String {
    let body_request = ExportRequest { title: String::new(), author: String::new(), ..request.clone() };
    let mut body = markdown_to_html(&compile_markdown(&body_request, &book.parts));

    if request.chapter_page_breaks {
        body = body.replace("<h1>", "<h1 class=\"page-break\">");
    }

    let title = html_escape(request.title.trim());
    let mut front = String::new();
    if !title.is_empty() {
        front.push_str(&format!("<h1 class=\"book-title\">{title}</h1>\n"));
        if !request.author.trim().is_empty() {
            front.push_str(&format!(
                "<p class=\"book-author\">{}</p>\n",
                html_escape(request.author.trim())
            ));
        }
        front.push_str("<div class=\"page-break\"></div>\n");
    }
    for m in &book.front {
        front.push_str(&matter_html(m));
    }
    for m in &book.back {
        body.push_str(&matter_html(m));
    }

    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{title}</title>\n<style>{BOOK_CSS}{PRINT_CSS}</style>\n</head>\n<body>\n{front}{body}</body>\n</html>\n"
    )
}

/// A matter page as its own section, starting a new page in print.
fn matter_html(m: &Matter) -> String {
    let (class, heading) = match m.kind.style() {
        MatterStyle::Small => (" matter-small", String::new()),
        MatterStyle::Centered => (" matter-centered", String::new()),
        MatterStyle::Headed(h) => ("", format!("<h2 class=\"matter-heading\">{}</h2>\n", html_escape(h))),
    };
    format!(
        "<section class=\"matter matter-{}{class} page-break\">\n{heading}{}</section>\n",
        m.kind.id(),
        markdown_to_html(&m.body())
    )
}

// ---------------------------------------------------------------------------
// EPUB
// ---------------------------------------------------------------------------

struct Chapter {
    title: String,
    markdown: String,
}

/// Splits the manuscript into EPUB files: a new one at every top-level
/// heading, or one per writing when there are no headings at all.
fn epub_chapters(request: &ExportRequest, parts: &[Part]) -> Vec<Chapter> {
    let has_headings = parts.iter().any(|p| matches!(p, Part::Heading { .. }));
    let top_level = parts
        .iter()
        .filter_map(|p| match p {
            Part::Heading { level, .. } => Some(*level),
            _ => None,
        })
        .min()
        .unwrap_or(1);

    let mut chapters: Vec<(String, Vec<&Part>)> = Vec::new();
    for part in parts {
        let starts_chapter = match part {
            Part::Heading { level, .. } => *level == top_level,
            Part::Document { .. } => !has_headings || chapters.is_empty(),
        };
        if starts_chapter || chapters.is_empty() {
            let title = match part {
                Part::Heading { text, .. } => text.clone(),
                Part::Document { title, label, .. } => title
                    .clone()
                    .filter(|t| !t.trim().is_empty())
                    .or_else(|| Some(label.clone()).filter(|l| !l.trim().is_empty()))
                    .unwrap_or_else(|| format!("Chapter {}", chapters.len() + 1)),
            };
            chapters.push((title, Vec::new()));
        }
        chapters.last_mut().expect("just pushed").1.push(part);
    }

    chapters
        .into_iter()
        .map(|(title, members)| {
            let owned: Vec<Part> = members.into_iter().cloned().collect();
            let chapter_request =
                ExportRequest { title: String::new(), author: String::new(), ..request.clone() };
            Chapter { title, markdown: compile_markdown(&chapter_request, &owned) }
        })
        .collect()
}

fn xhtml_page(title: &str, body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n<head>\n<meta charset=\"utf-8\"/>\n<title>{}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"stylesheet.css\"/>\n</head>\n<body>\n{}</body>\n</html>\n",
        html_escape(title),
        body
    )
}

fn render_epub(request: &ExportRequest, book: &Book) -> Result<Vec<u8>, String> {
    let err = |e: epub_builder::Error| format!("EPUB error: {e}");
    let mut builder = EpubBuilder::new(ZipLibrary::new().map_err(err)?).map_err(err)?;
    builder.epub_version(epub_builder::EpubVersion::V30);
    let title = if request.title.trim().is_empty() { "Untitled" } else { request.title.trim() };
    builder.set_title(title);
    if !request.author.trim().is_empty() {
        builder.add_author(request.author.trim());
    }
    builder.set_generator("Memorie");
    builder.set_languages(vec![request.language().to_string()]);
    builder.stylesheet(BOOK_CSS.as_bytes()).map_err(err)?;
    builder.set_toc_name("Contents");

    let mut title_page = format!("<h1 class=\"book-title\">{}</h1>\n", html_escape(title));
    if !request.author.trim().is_empty() {
        title_page.push_str(&format!("<p class=\"book-author\">{}</p>\n", html_escape(request.author.trim())));
    }
    builder
        .add_content(
            EpubContent::new("title.xhtml", xhtml_page(title, &title_page).as_bytes())
                .title(title)
                .reftype(epub_builder::ReferenceType::TitlePage),
        )
        .map_err(err)?;
    let add_matter = |builder: &mut EpubBuilder<ZipLibrary>, place: &str, pages: &[Matter]| {
        for (i, m) in pages.iter().enumerate() {
            // Only headed pages get a title, and so an entry in the contents.
            let title = match m.kind.style() {
                MatterStyle::Headed(h) => h,
                _ => "",
            };
            let page = xhtml_page(if title.is_empty() { m.kind.id() } else { title }, &matter_html(m));
            builder
                .add_content(
                    EpubContent::new(format!("{place}_{:02}_{}.xhtml", i + 1, m.kind.id()), page.as_bytes())
                        .title(title)
                        .reftype(m.kind.epub_reftype()),
                )
                .map_err(err)?;
        }
        Ok::<_, String>(())
    };
    add_matter(&mut builder, "front", &book.front)?;
    // Goes where it's called: after the title page and front matter.
    if request.table_of_contents {
        builder.inline_toc();
    }

    for (i, chapter) in epub_chapters(request, &book.parts).iter().enumerate() {
        let body = markdown_to_html(&chapter.markdown);
        let page = xhtml_page(&chapter.title, &body);
        builder
            .add_content(
                EpubContent::new(format!("chapter_{:03}.xhtml", i + 1), page.as_bytes())
                    .title(chapter.title.as_str())
                    .reftype(epub_builder::ReferenceType::Text),
            )
            .map_err(err)?;
    }
    add_matter(&mut builder, "back", &book.back)?;

    let mut out = Vec::new();
    builder.generate(&mut out).map_err(err)?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// DOCX
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Marks {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
}

struct DocxWriter {
    docx: Docx,
    manuscript: bool,
    chapter_page_breaks: bool,
    /// Runs of the paragraph being built, if any.
    runs: Vec<Run>,
    marks: Marks,
    heading: Option<usize>,
    quote_depth: usize,
    /// Stack of lists: `Some(n)` for ordered lists (next number), `None` for bullets.
    lists: Vec<Option<u64>>,
    in_code_block: bool,
    code_block: String,
    /// The first top-level heading doesn't need a page break before it.
    seen_top_heading: bool,
    /// Set while writing a page of front or back matter.
    matter: Option<MatterStyle>,
}

/// Twentieths of a point, the unit DOCX uses for spacing and indents.
const TWIPS_PER_INCH: i32 = 1440;

impl DocxWriter {
    fn new(manuscript: bool, chapter_page_breaks: bool) -> Self {
        let body_font = if manuscript { "Times New Roman" } else { "Georgia" };
        let fonts = RunFonts::new().ascii(body_font).hi_ansi(body_font).cs(body_font).east_asia(body_font);
        let mut docx = Docx::new()
            .default_fonts(fonts)
            .default_size(24) // half-points: 12pt
            .page_margin(
                PageMargin::new()
                    .top(TWIPS_PER_INCH)
                    .bottom(TWIPS_PER_INCH)
                    .left(TWIPS_PER_INCH)
                    .right(TWIPS_PER_INCH),
            );

        let heading_sizes = [40usize, 32, 28, 26, 24, 24];
        for (i, size) in heading_sizes.iter().enumerate() {
            let level = i + 1;
            let mut style = Style::new(format!("Heading{level}"), StyleType::Paragraph)
                .name(format!("Heading {level}"))
                .size(*size)
                .outline_lvl(i)
                .line_spacing(LineSpacing::new().before(360).after(240));
            if manuscript {
                style = style.align(AlignmentType::Center);
            } else {
                style = style.bold();
            }
            docx = docx.add_style(style);
        }

        DocxWriter {
            docx,
            manuscript,
            chapter_page_breaks,
            runs: Vec::new(),
            marks: Marks::default(),
            heading: None,
            quote_depth: 0,
            lists: Vec::new(),
            in_code_block: false,
            code_block: String::new(),
            seen_top_heading: false,
            matter: None,
        }
    }

    fn body_paragraph(&self) -> Paragraph {
        let mut p = Paragraph::new();
        if matches!(self.matter, Some(MatterStyle::Small | MatterStyle::Centered)) {
            p = p.line_spacing(LineSpacing::new().after(160));
            if self.matter == Some(MatterStyle::Centered) {
                p = p.align(AlignmentType::Center);
            }
        } else if self.manuscript {
            // Double spaced, half-inch first line indent, no gap between paragraphs.
            p = p
                .line_spacing(LineSpacing::new().line(480).after(0))
                .indent(None, Some(SpecialIndentType::FirstLine(TWIPS_PER_INCH / 2)), None, None);
        } else {
            p = p.line_spacing(LineSpacing::new().line(300).after(160));
        }
        p
    }

    fn text_run(&self, text: &str) -> Run {
        let mut run = Run::new().add_text(text);
        if self.marks.bold {
            run = run.bold();
        }
        if self.marks.italic {
            run = run.italic();
        }
        if self.marks.strike {
            run = run.strike();
        }
        if self.marks.code {
            run = run.fonts(RunFonts::new().ascii("Courier New").hi_ansi("Courier New"));
        }
        match self.matter {
            Some(MatterStyle::Small) => run = run.size(20),
            Some(MatterStyle::Centered) => run = run.italic(),
            _ => {}
        }
        run
    }

    fn flush_paragraph(&mut self) {
        if self.runs.is_empty() {
            return;
        }
        let runs = std::mem::take(&mut self.runs);
        let mut p = if let Some(level) = self.heading {
            let mut p = Paragraph::new().style(&format!("Heading{level}")).keep_next(true);
            if level == 1 && self.chapter_page_breaks {
                if self.seen_top_heading {
                    p = p.page_break_before(true);
                }
                self.seen_top_heading = true;
            }
            p
        } else {
            let mut p = self.body_paragraph();
            let list_depth = self.lists.len() as i32;
            let left = TWIPS_PER_INCH / 2 * (self.quote_depth as i32 + list_depth);
            if left > 0 {
                p = p.indent(Some(left), None, None, None);
            }
            if self.quote_depth > 0 {
                p = p.italic();
            }
            p
        };
        for run in runs {
            p = p.add_run(run);
        }
        self.docx = std::mem::take(&mut self.docx).add_paragraph(p);
    }

    fn centered_line(&mut self, text: &str) {
        self.flush_paragraph();
        let p = Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(LineSpacing::new().before(240).after(240))
            .add_run(Run::new().add_text(text));
        self.docx = std::mem::take(&mut self.docx).add_paragraph(p);
    }

    fn page_break(&mut self) {
        self.flush_paragraph();
        let p = Paragraph::new().add_run(Run::new().add_break(BreakType::Page));
        self.docx = std::mem::take(&mut self.docx).add_paragraph(p);
    }

    /// One page of front or back matter. The caller breaks the page.
    fn write_matter(&mut self, m: &Matter) {
        let style = m.kind.style();
        match style {
            MatterStyle::Headed(heading) => {
                let p = Paragraph::new()
                    .align(AlignmentType::Center)
                    .keep_next(true)
                    .line_spacing(LineSpacing::new().before(TWIPS_PER_INCH as u32).after(480))
                    .add_run(Run::new().add_text(heading).size(40));
                self.docx = std::mem::take(&mut self.docx).add_paragraph(p);
            }
            MatterStyle::Centered => {
                // Set a third of the way down the page.
                let p = Paragraph::new().line_spacing(LineSpacing::new().before(TWIPS_PER_INCH as u32 * 2));
                self.docx = std::mem::take(&mut self.docx).add_paragraph(p);
            }
            MatterStyle::Small => {}
        }
        self.matter = Some(style);
        self.write_markdown(&m.body());
        self.matter = None;
    }

    fn write_markdown(&mut self, markdown: &str) {
        for event in clean_events(markdown) {
            self.event(event);
        }
        self.flush_paragraph();
    }

    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => self.flush_paragraph(),
                Tag::Heading { level, .. } => {
                    self.flush_paragraph();
                    self.heading = Some(heading_number(level));
                }
                Tag::BlockQuote(_) => {
                    self.flush_paragraph();
                    self.quote_depth += 1;
                }
                Tag::CodeBlock(_) => {
                    self.flush_paragraph();
                    self.in_code_block = true;
                    self.code_block.clear();
                }
                Tag::List(start) => {
                    self.flush_paragraph();
                    self.lists.push(start);
                }
                Tag::Item => {
                    self.flush_paragraph();
                    let marker = match self.lists.last_mut() {
                        Some(Some(n)) => {
                            let marker = format!("{n}. ");
                            *n += 1;
                            marker
                        }
                        _ => "• ".to_string(),
                    };
                    self.runs.push(Run::new().add_text(marker));
                }
                Tag::Emphasis => self.marks.italic = true,
                Tag::Strong => self.marks.bold = true,
                Tag::Strikethrough => self.marks.strike = true,
                Tag::TableRow | Tag::TableHead => self.flush_paragraph(),
                Tag::TableCell => {
                    if !self.runs.is_empty() {
                        self.runs.push(Run::new().add_tab());
                    }
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph | TagEnd::Item | TagEnd::TableRow | TagEnd::TableHead => {
                    self.flush_paragraph()
                }
                TagEnd::Heading(_) => {
                    self.flush_paragraph();
                    self.heading = None;
                }
                TagEnd::BlockQuote(_) => {
                    self.flush_paragraph();
                    self.quote_depth = self.quote_depth.saturating_sub(1);
                }
                TagEnd::CodeBlock => {
                    self.in_code_block = false;
                    let code = std::mem::take(&mut self.code_block);
                    for line in code.trim_end_matches('\n').lines() {
                        let p = Paragraph::new()
                            .line_spacing(LineSpacing::new().line(240).after(0))
                            .indent(Some(TWIPS_PER_INCH / 2), None, None, None)
                            .add_run(
                                Run::new()
                                    .add_text(line)
                                    .fonts(RunFonts::new().ascii("Courier New").hi_ansi("Courier New"))
                                    .size(20),
                            );
                        self.docx = std::mem::take(&mut self.docx).add_paragraph(p);
                    }
                }
                TagEnd::List(_) => {
                    self.flush_paragraph();
                    self.lists.pop();
                }
                TagEnd::Emphasis => self.marks.italic = false,
                TagEnd::Strong => self.marks.bold = false,
                TagEnd::Strikethrough => self.marks.strike = false,
                _ => {}
            },
            Event::Text(text) => {
                if self.in_code_block {
                    self.code_block.push_str(&text);
                } else {
                    let run = self.text_run(&text);
                    self.runs.push(run);
                }
            }
            Event::Code(text) => {
                let saved = self.marks;
                self.marks.code = true;
                let run = self.text_run(&text);
                self.runs.push(run);
                self.marks = saved;
            }
            Event::Html(raw) | Event::InlineHtml(raw) => {
                if let Some(inner) = raw
                    .trim()
                    .strip_prefix("<p class=\"separator\">")
                    .and_then(|s| s.strip_suffix("</p>"))
                {
                    let text = inner.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"");
                    self.centered_line(&text);
                }
            }
            Event::SoftBreak => self.runs.push(Run::new().add_text(" ")),
            Event::HardBreak => self.runs.push(Run::new().add_break(BreakType::TextWrapping)),
            Event::Rule => self.centered_line("* * *"),
            _ => {}
        }
    }
}

fn heading_number(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn render_docx(request: &ExportRequest, book: &Book) -> Result<Vec<u8>, String> {
    let parts = &book.parts;
    let mut writer = DocxWriter::new(request.manuscript_format, request.chapter_page_breaks);

    let title = request.title.trim();
    if !title.is_empty() {
        let mut p = Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(LineSpacing::new().before(TWIPS_PER_INCH as u32 * 3).after(240))
            .add_run(Run::new().add_text(title).size(48));
        if !writer.manuscript {
            p = p.bold();
        }
        writer.docx = std::mem::take(&mut writer.docx).add_paragraph(p);
        if !request.author.trim().is_empty() {
            let p = Paragraph::new()
                .align(AlignmentType::Center)
                .add_run(Run::new().add_text(request.author.trim()).italic());
            writer.docx = std::mem::take(&mut writer.docx).add_paragraph(p);
        }
        writer.page_break();
    }

    for m in &book.front {
        writer.write_matter(m);
        writer.page_break();
    }

    let entries = toc_entries(parts);
    if request.table_of_contents && !entries.is_empty() {
        let depth = entries.iter().map(|(level, _)| *level).max().unwrap_or(1);
        // Word fills in page numbers when it opens the file (the field is
        // marked dirty); other apps show the titles as they are.
        let mut toc = TableOfContents::new()
            .heading_styles_range(1, depth)
            .alias("Table of contents")
            .dirty()
            .add_before_paragraph(
                Paragraph::new()
                    .align(AlignmentType::Center)
                    .line_spacing(LineSpacing::new().after(480))
                    .add_run(Run::new().add_text("Contents").size(32)),
            );
        for (level, text) in &entries {
            toc = toc.add_item(TableOfContentsItem::new().text(text).level(*level).page_ref(""));
        }
        writer.docx = std::mem::take(&mut writer.docx).add_table_of_contents(toc);
        writer.page_break();
    }

    if let Some(head) = request.running_head_text() {
        let mut run = Run::new();
        if !head.is_empty() {
            run = run.add_text(format!("{head} / "));
        }
        let run = run
            .add_field_char(FieldCharType::Begin, false)
            .add_instr_text(InstrText::PAGE(InstrPAGE::new()))
            .add_field_char(FieldCharType::Separate, false)
            .add_text("1")
            .add_field_char(FieldCharType::End, false);
        let header = Header::new().add_paragraph(Paragraph::new().align(AlignmentType::Right).add_run(run));
        writer.docx = std::mem::take(&mut writer.docx).header(header);
        if !title.is_empty() {
            // Nothing on the title page.
            writer.docx = std::mem::take(&mut writer.docx).first_header(Header::new());
        }
    }

    let body_request = ExportRequest { title: String::new(), author: String::new(), ..request.clone() };
    writer.write_markdown(&compile_markdown(&body_request, parts));
    for m in &book.back {
        writer.page_break();
        writer.write_matter(m);
    }

    let mut out = std::io::Cursor::new(Vec::new());
    writer
        .docx
        .build()
        .pack(&mut out)
        .map_err(|e| format!("DOCX error: {e}"))?;
    Ok(out.into_inner())
}

// ---------------------------------------------------------------------------
// PDF
// ---------------------------------------------------------------------------

fn render_pdf(request: &ExportRequest, book: &Book) -> Result<Vec<u8>, String> {
    let parts = &book.parts;
    let body_request = ExportRequest { title: String::new(), author: String::new(), ..request.clone() };
    let markdown = compile_markdown(&body_request, parts);
    let mut body = crate::pdf::events_to_typst(clean_events(&markdown));
    for m in &book.back {
        body.push_str(&matter_typst(m));
    }
    let front: String = book.front.iter().map(matter_typst).collect();
    let layout = crate::pdf::PdfLayout {
        title: request.title.trim(),
        author: request.author.trim(),
        page_size: request.page_size,
        manuscript: request.manuscript_format,
        chapter_page_breaks: request.chapter_page_breaks,
        lang: request.language(),
        toc_depth: request
            .table_of_contents
            .then(|| toc_entries(parts).iter().map(|(level, _)| *level).max())
            .flatten(),
        running_head: request.running_head_text(),
        front: &front,
    };
    crate::pdf::typeset(&layout, &body)
}

/// A matter page in Typst. Small and centered pages go without running head
/// or page number, as in print.
fn matter_typst(m: &Matter) -> String {
    let body = crate::pdf::events_to_typst(clean_events(&m.body()));
    match m.kind.style() {
        MatterStyle::Small => format!(
            "#page(header: none, footer: none)[#v(1fr)\n#set text(size: 0.85em)\n#set par(first-line-indent: 0em, justify: false)\n{body}]\n\n"
        ),
        MatterStyle::Centered => format!(
            "#page(header: none, footer: none)[#v(25%)\n#set align(center)\n#set text(style: \"italic\")\n#set par(first-line-indent: 0em, justify: false)\n{body}]\n\n"
        ),
        MatterStyle::Headed(heading) => format!(
            "#pagebreak(weak: true)\n#align(center, text(size: 1.6em)[{}])\n#v(1.5em)\n{body}\n#pagebreak(weak: true)\n\n",
            crate::pdf::escape_text(heading)
        ),
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Characters that are awkward or invalid in file names on some platform.
fn file_stem(title: &str) -> String {
    let cleaned: String = title
        .trim()
        .chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '-' } else { c })
        .collect();
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace()).to_string();
    if cleaned.is_empty() { "Untitled".to_string() } else { cleaned }
}

/// `dir/stem.ext`, or `dir/stem 2.ext` and so on if that is taken.
fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem} {n}.{ext}")))
        .find(|p| !p.exists())
        .expect("unbounded range always finds a free name")
}

/// Per-binder compile settings live in `<binder>/.compile.json`, so they move
/// and sync with the binder. Hidden files stay out of the binder tree and index.
const COMPILE_SETTINGS_FILE: &str = ".compile.json";

fn compile_settings_path(content_dir: &Path, binder: &str) -> Result<PathBuf, String> {
    if binder.is_empty() || binder.contains(['/', '\\']) {
        return Err(format!("Invalid binder: {binder}"));
    }
    Ok(safe_join(content_dir, binder)?.join(COMPILE_SETTINGS_FILE))
}

/// The binder's saved settings as JSON, or `None` if it has none yet.
pub fn read_compile_settings(content_dir: &Path, binder: &str) -> Result<Option<String>, String> {
    let path = compile_settings_path(content_dir, binder)?;
    match std::fs::read_to_string(&path) {
        Ok(json) => Ok(Some(json)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Could not read {}: {e}", path.display())),
    }
}

/// Saves the binder's settings. `settings` must be a JSON object.
pub fn write_compile_settings(content_dir: &Path, binder: &str, settings: &serde_json::Value) -> Result<(), String> {
    if !settings.is_object() {
        return Err("Compile settings must be an object".into());
    }
    let path = compile_settings_path(content_dir, binder)?;
    if !path.parent().is_some_and(Path::is_dir) {
        return Err(format!("No binder named \"{binder}\""));
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json + "\n").map_err(|e| format!("Could not write {}: {e}", path.display()))
}

/// Renders the manuscript and writes it to `out_dir`. Returns the file written.
pub fn export(request: &ExportRequest, content_dir: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let book = load_book(request, content_dir)?;
    let bytes = match request.format {
        ExportFormat::Html => render_html(request, &book).into_bytes(),
        ExportFormat::Markdown => compile_book_markdown(request, &book).into_bytes(),
        ExportFormat::Docx => render_docx(request, &book)?,
        ExportFormat::Epub => render_epub(request, &book)?,
        ExportFormat::Pdf => render_pdf(request, &book)?,
    };

    std::fs::create_dir_all(out_dir).map_err(|e| format!("Could not create {}: {e}", out_dir.display()))?;
    let path = unique_path(out_dir, &file_stem(&request.title), request.format.extension());
    std::fs::write(&path, bytes).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn request(format: ExportFormat, items: Vec<ExportItem>) -> ExportRequest {
        ExportRequest {
            format,
            title: "My Novel".into(),
            author: "A. Writer".into(),
            items,
            scene_separator: "* * *".into(),
            manuscript_format: true,
            chapter_page_breaks: true,
            page_size: Default::default(),
            lang: None,
            chapter_heading: String::new(),
            exclude_statuses: Vec::new(),
            table_of_contents: false,
            running_head: false,
            front_matter: Vec::new(),
            back_matter: Vec::new(),
        }
    }

    fn sample(dir: &Path) -> Vec<ExportItem> {
        std::fs::create_dir_all(dir.join("Novel/Chapter 1")).unwrap();
        std::fs::write(
            dir.join("Novel/Chapter 1/Scene 1.md"),
            "It was a **dark** and _stormy_ night. See [Scene 2](#writing/Novel/Chapter%201/Scene%202.md).\n\n- one\n- two\n",
        )
        .unwrap();
        std::fs::write(dir.join("Novel/Chapter 1/Scene 2.md"), "> A quote <b>raw</b>\n\n```\ncode & more\n```\n").unwrap();
        vec![
            ExportItem::Heading { level: 1, text: "Chapter 1".into() },
            ExportItem::Document { name: "Novel/Chapter 1/Scene 1.md".into(), title: None },
            ExportItem::Document { name: "Novel/Chapter 1/Scene 2.md".into(), title: Some("Scene 2".into()) },
        ]
    }

    #[test]
    fn markdown_compile_orders_and_separates() {
        let dir = tempdir().unwrap();
        let items = sample(dir.path());
        let req = request(ExportFormat::Markdown, items);
        let parts = load_parts(dir.path(), &req.items).unwrap();
        let md = compile_markdown(&req, &parts);
        let chapter = md.find("## Chapter 1").unwrap();
        let scene1 = md.find("dark").unwrap();
        let separator = md.find("<p class=\"separator\">* * *</p>").unwrap();
        let scene2 = md.find("### Scene 2").unwrap();
        assert!(chapter < scene1 && scene1 < separator && separator < scene2);
    }

    #[test]
    fn html_drops_internal_links_and_escapes_raw_html() {
        let dir = tempdir().unwrap();
        let req = request(ExportFormat::Html, sample(dir.path()));
        let html = render_html(&req, &load_book(&req, dir.path()).unwrap());
        assert!(!html.contains("#writing/"));
        assert!(html.contains("Scene 2"));
        assert!(html.contains("&lt;b&gt;raw"));
        assert!(html.contains("<strong>dark</strong>"));
        assert!(html.contains("class=\"separator\""));
    }

    #[test]
    fn writes_every_format() {
        let dir = tempdir().unwrap();
        let out = tempdir().unwrap();
        let items = sample(dir.path());
        for format in [ExportFormat::Docx, ExportFormat::Epub, ExportFormat::Html, ExportFormat::Markdown, ExportFormat::Pdf] {
            let path = export(&request(format, items.clone()), dir.path(), out.path()).unwrap();
            assert!(path.exists(), "{format:?} not written");
            assert!(std::fs::metadata(&path).unwrap().len() > 0);
        }
        // Zip containers start with "PK".
        let docx = std::fs::read(out.path().join("My Novel.docx")).unwrap();
        assert_eq!(&docx[..2], b"PK");
        let epub = std::fs::read(out.path().join("My Novel.epub")).unwrap();
        assert_eq!(&epub[..2], b"PK");
    }

    #[test]
    fn second_export_gets_a_new_name() {
        let dir = tempdir().unwrap();
        let out = tempdir().unwrap();
        let items = sample(dir.path());
        let a = export(&request(ExportFormat::Markdown, items.clone()), dir.path(), out.path()).unwrap();
        let b = export(&request(ExportFormat::Markdown, items), dir.path(), out.path()).unwrap();
        assert_ne!(a, b);
        assert!(b.ends_with("My Novel 2.md"));
    }

    #[test]
    fn front_matter_is_not_exported() {
        assert_eq!(strip_front_matter("---\nstatus: \"Done\"\n---\n\nBody"), "Body");
        assert_eq!(strip_front_matter("---\ntags:\n  - a\n---\nBody"), "Body");
        assert_eq!(strip_front_matter("---\n\nNot metadata\n---\n"), "---\n\nNot metadata\n---\n");
        let prose = "---\ntitle: x\nSome prose here.\n---\nMore";
        assert_eq!(strip_front_matter(prose), prose);
        assert_eq!(strip_front_matter("Plain"), "Plain");
    }

    #[test]
    fn headings_in_writings_sit_below_structure() {
        let md = "# Inside\n\n```\n# not a heading\n```\n\n#hashtag\n\n###### Deep";
        let out = demote_headings(md, 2);
        assert!(out.starts_with("### Inside"), "{out}");
        assert!(out.contains("\n# not a heading\n"), "{out}");
        assert!(out.contains("#hashtag"), "{out}");
        assert!(out.contains("###### Deep"), "{out}");
    }

    #[test]
    fn folder_names_are_escaped() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# Scene heading\n\nText").unwrap();
        let items = vec![
            ExportItem::Heading { level: 1, text: "Part *One* <draft>".into() },
            ExportItem::Document { name: "a.md".into(), title: None },
        ];
        let req = ExportRequest { title: String::new(), ..request(ExportFormat::Markdown, items) };
        let parts = load_parts(dir.path(), &req.items).unwrap();
        let md = compile_markdown(&req, &parts);
        assert!(md.starts_with("# Part \\*One\\* \\<draft\\>"), "{md}");
        assert!(md.contains("## Scene heading"), "{md}");
    }

    fn headings(parts: &[Part]) -> Vec<String> {
        parts
            .iter()
            .filter_map(|p| match p {
                Part::Heading { text, .. } => Some(text.clone()),
                Part::Document { title, .. } => title.clone(),
            })
            .collect()
    }

    #[test]
    fn chapter_headings_number_innermost_folders() {
        let dir = tempdir().unwrap();
        for f in ["a.md", "b.md", "c.md"] {
            std::fs::write(dir.path().join(f), "Text").unwrap();
        }
        let doc = |n: &str| ExportItem::Document { name: n.into(), title: None };
        let items = vec![
            ExportItem::Heading { level: 1, text: "Part One".into() },
            ExportItem::Heading { level: 2, text: "Arrival".into() },
            doc("a.md"),
            ExportItem::Heading { level: 2, text: "Storm".into() },
            doc("b.md"),
            ExportItem::Heading { level: 1, text: "Part Two".into() },
            ExportItem::Heading { level: 2, text: "After".into() },
            doc("c.md"),
        ];
        let req = ExportRequest {
            chapter_heading: "Chapter {word} ({roman}): {title}".into(),
            ..request(ExportFormat::Markdown, items)
        };
        let parts = load_manuscript(&req, dir.path()).unwrap();
        assert_eq!(
            headings(&parts),
            ["Part One", "Chapter One (I): Arrival", "Chapter Two (II): Storm", "Part Two", "Chapter Three (III): After"]
        );
    }

    #[test]
    fn chapter_headings_without_folders_use_writings() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("Opening.md"), "Text").unwrap();
        std::fs::write(dir.path().join("b.md"), "Text").unwrap();
        let items = vec![
            ExportItem::Document { name: "Opening.md".into(), title: None },
            ExportItem::Document { name: "b.md".into(), title: Some("The Road".into()) },
        ];
        let req = ExportRequest { chapter_heading: "{n}. {title}".into(), ..request(ExportFormat::Markdown, items) };
        let parts = load_manuscript(&req, dir.path()).unwrap();
        assert_eq!(headings(&parts), ["1. Opening", "2. The Road"]);
    }

    #[test]
    fn status_filter_drops_writings_and_empty_folders() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "---\nstatus: \"Done\"\n---\nKeep").unwrap();
        std::fs::write(dir.path().join("b.md"), "---\nsynopsis: x\nstatus: to do\n---\nSkip").unwrap();
        std::fs::write(dir.path().join("c.md"), "No metadata").unwrap();
        let items = vec![
            ExportItem::Heading { level: 1, text: "One".into() },
            ExportItem::Document { name: "a.md".into(), title: None },
            ExportItem::Heading { level: 1, text: "Two".into() },
            ExportItem::Heading { level: 2, text: "Inner".into() },
            ExportItem::Document { name: "b.md".into(), title: None },
            ExportItem::Heading { level: 1, text: "Three".into() },
            ExportItem::Document { name: "c.md".into(), title: None },
        ];
        let req = ExportRequest {
            exclude_statuses: vec!["To Do".into()],
            chapter_heading: "Chapter {n}".into(),
            ..request(ExportFormat::Markdown, items)
        };
        let parts = load_manuscript(&req, dir.path()).unwrap();
        assert_eq!(headings(&parts), ["Chapter 1", "Chapter 2"]);
        let md = compile_markdown(&req, &parts);
        assert!(md.contains("Keep") && md.contains("No metadata") && !md.contains("Skip"), "{md}");

        let all_out = ExportRequest { exclude_statuses: vec!["done".into(), "to do".into()], ..req.clone() };
        let only_meta = ExportRequest {
            items: all_out.items[..5].to_vec(),
            ..all_out
        };
        assert!(load_manuscript(&only_meta, dir.path()).is_err());
    }

    #[test]
    fn number_formats() {
        assert_eq!(number_word(21), "Twenty-One");
        assert_eq!(number_word(40), "Forty");
        assert_eq!(number_word(120), "120");
        assert_eq!(roman(1994), "MCMXCIV");
        assert_eq!(front_matter_value("---\nstatus: 'It''s done'\n---\nx", "status").as_deref(), Some("It's done"));
        assert_eq!(front_matter_value("No block", "status"), None);
    }

    fn unzip(bytes: &[u8], name: &str) -> String {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut file = archive.by_name(name).unwrap();
        let mut out = String::new();
        std::io::Read::read_to_string(&mut file, &mut out).unwrap();
        out
    }

    #[test]
    fn contents_and_running_heads() {
        let dir = tempdir().unwrap();
        let items = sample(dir.path());
        let req = ExportRequest {
            table_of_contents: true,
            running_head: true,
            author: "Ann Writer".into(),
            ..request(ExportFormat::Docx, items)
        };
        assert_eq!(req.running_head_text().as_deref(), Some("Writer / MY NOVEL"));
        let book = load_book(&req, dir.path()).unwrap();

        let docx = render_docx(&req, &book).unwrap();
        let document = unzip(&docx, "word/document.xml");
        assert!(document.contains("TOC \\o &quot;1-1&quot;") || document.contains("TOC \\o \"1-1\""), "{document}");
        assert!(document.contains("titlePg"), "title page should have no header");
        let headers: String = (1..=2).map(|i| unzip(&docx, &format!("word/header{i}.xml"))).collect();
        assert!(headers.contains("Writer / MY NOVEL / ") && headers.contains("PAGE"), "{headers}");

        let epub = render_epub(&ExportRequest { format: ExportFormat::Epub, ..req.clone() }, &book).unwrap();
        assert!(unzip(&epub, "OEBPS/toc.xhtml").contains("Chapter 1"));

        let pdf = render_pdf(&ExportRequest { format: ExportFormat::Pdf, ..req }, &book).unwrap();
        assert_eq!(&pdf[..4], b"%PDF");
    }

    #[test]
    fn front_and_back_matter() {
        let dir = tempdir().unwrap();
        let items = sample(dir.path());
        std::fs::write(dir.path().join("Thanks.md"), "---\nstatus: Done\n---\nThank you, {author}.").unwrap();
        let text = |kind, text: &str| MatterItem { kind, text: text.into(), name: None };
        let req = ExportRequest {
            table_of_contents: true,
            front_matter: vec![
                text(MatterKind::Copyright, "Copyright © {year} {author}"),
                text(MatterKind::Dedication, "For *Sam*"),
                text(MatterKind::Epigraph, "   "),
            ],
            back_matter: vec![MatterItem { kind: MatterKind::Acknowledgments, text: String::new(), name: Some("Thanks.md".into()) }],
            ..request(ExportFormat::Markdown, items)
        };
        let book = load_book(&req, dir.path()).unwrap();
        assert_eq!(book.front.len(), 2, "empty pages are skipped");
        let year = chrono::Local::now().format("%Y").to_string();

        let md = compile_book_markdown(&req, &book);
        let copyright = md.find(&format!("Copyright © {year} A. Writer")).unwrap();
        let dedication = md.find("For *Sam*").unwrap();
        let chapter = md.find("## Chapter 1").unwrap();
        let thanks = md.find("## Acknowledgments\n\nThank you, A. Writer.").unwrap();
        assert!(md.starts_with("# My Novel") && copyright < dedication && dedication < chapter && chapter < thanks, "{md}");
        assert!(!md.contains("status:"));

        let html = render_html(&req, &book);
        assert!(html.contains("matter-dedication matter-centered page-break"), "{html}");
        assert!(html.contains("<h2 class=\"matter-heading\">Acknowledgments</h2>"));

        let docx = render_docx(&req, &book).unwrap();
        let document = unzip(&docx, "word/document.xml");
        let (c, t, a) = (document.find("Copyright ©").unwrap(), document.find("TOC").unwrap(), document.find("Acknowledgments").unwrap());
        assert!(c < t && t < a, "front matter before contents, back matter at the end");

        let epub = render_epub(&req, &book).unwrap();
        let nav = unzip(&epub, "OEBPS/toc.xhtml");
        assert!(nav.contains("Acknowledgments") && !nav.contains("copyright"), "{nav}");
        assert!(unzip(&epub, "OEBPS/front_02_dedication.xhtml").contains("Sam"));

        let pdf = render_pdf(&req, &book).unwrap();
        assert_eq!(&pdf[..4], b"%PDF");
    }

    #[test]
    fn compile_settings_round_trip() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Novel")).unwrap();
        assert_eq!(read_compile_settings(dir.path(), "Novel").unwrap(), None);
        let settings = serde_json::json!({ "version": 1, "excluded": ["Notes"] });
        write_compile_settings(dir.path(), "Novel", &settings).unwrap();
        let back: serde_json::Value = serde_json::from_str(&read_compile_settings(dir.path(), "Novel").unwrap().unwrap()).unwrap();
        assert_eq!(back, settings);
        assert!(write_compile_settings(dir.path(), "Missing", &settings).is_err());
        assert!(write_compile_settings(dir.path(), "../x", &settings).is_err());
        assert!(write_compile_settings(dir.path(), "Novel", &serde_json::json!([1])).is_err());
    }

    #[test]
    fn rejects_paths_outside_content() {
        let dir = tempdir().unwrap();
        let items = vec![ExportItem::Document { name: "../secret.md".into(), title: None }];
        assert!(load_parts(dir.path(), &items).is_err());
    }
}


