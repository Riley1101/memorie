//! Compiles writings into a single manuscript: DOCX, PDF, EPUB, standalone
//! HTML, or one combined Markdown file.
//!
//! The frontend decides what goes in and in which order (it already knows the
//! binder tree as the writer sees it) and sends a flat list of items: folder
//! headings and documents. This module reads the documents and renders them.

use docx_rs::{
    AlignmentType, BreakType, Docx, LineSpacing, PageMargin, Paragraph, Run, RunFonts,
    SpecialIndentType, Style, StyleType,
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
}

impl ExportRequest {
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
    Document { title: Option<String>, label: String, markdown: String },
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
                let markdown = strip_front_matter(&markdown).to_string();
                let label = Path::new(name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Ok(Part::Document { title: title.clone(), label, markdown })
            }
        })
        .collect()
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
pub fn render_html(request: &ExportRequest, content_dir: &Path) -> Result<String, String> {
    let parts = load_parts(content_dir, &request.items)?;
    let body_request = ExportRequest { title: String::new(), author: String::new(), ..request.clone() };
    let mut body = markdown_to_html(&compile_markdown(&body_request, &parts));

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

    Ok(format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{title}</title>\n<style>{BOOK_CSS}{PRINT_CSS}</style>\n</head>\n<body>\n{front}{body}</body>\n</html>\n"
    ))
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

fn render_epub(request: &ExportRequest, parts: &[Part]) -> Result<Vec<u8>, String> {
    let err = |e: epub_builder::Error| format!("EPUB error: {e}");
    let mut builder = EpubBuilder::new(ZipLibrary::new().map_err(err)?).map_err(err)?;
    builder.epub_version(epub_builder::EpubVersion::V30);
    let title = if request.title.trim().is_empty() { "Untitled" } else { request.title.trim() };
    builder.set_title(title);
    if !request.author.trim().is_empty() {
        builder.add_author(request.author.trim());
    }
    builder.set_generator("Memorie");
    builder.set_lang(request.language());
    builder.stylesheet(BOOK_CSS.as_bytes()).map_err(err)?;

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

    for (i, chapter) in epub_chapters(request, parts).iter().enumerate() {
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
        }
    }

    fn body_paragraph(&self) -> Paragraph {
        let mut p = Paragraph::new();
        if self.manuscript {
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

fn render_docx(request: &ExportRequest, parts: &[Part]) -> Result<Vec<u8>, String> {
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

    let body_request = ExportRequest { title: String::new(), author: String::new(), ..request.clone() };
    writer.write_markdown(&compile_markdown(&body_request, parts));

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

fn render_pdf(request: &ExportRequest, parts: &[Part]) -> Result<Vec<u8>, String> {
    let body_request = ExportRequest { title: String::new(), author: String::new(), ..request.clone() };
    let markdown = compile_markdown(&body_request, parts);
    let body = crate::pdf::events_to_typst(clean_events(&markdown));
    let layout = crate::pdf::PdfLayout {
        title: request.title.trim(),
        author: request.author.trim(),
        page_size: request.page_size,
        manuscript: request.manuscript_format,
        chapter_page_breaks: request.chapter_page_breaks,
        lang: request.language(),
    };
    crate::pdf::typeset(&layout, &body)
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

/// Renders the manuscript and writes it to `out_dir`. Returns the file written.
pub fn export(request: &ExportRequest, content_dir: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let bytes = match request.format {
        ExportFormat::Html => render_html(request, content_dir)?.into_bytes(),
        ExportFormat::Markdown => {
            let parts = load_parts(content_dir, &request.items)?;
            compile_markdown(request, &parts).into_bytes()
        }
        ExportFormat::Docx => render_docx(request, &load_parts(content_dir, &request.items)?)?,
        ExportFormat::Epub => render_epub(request, &load_parts(content_dir, &request.items)?)?,
        ExportFormat::Pdf => render_pdf(request, &load_parts(content_dir, &request.items)?)?,
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
        let html = render_html(&req, dir.path()).unwrap();
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

    #[test]
    fn rejects_paths_outside_content() {
        let dir = tempdir().unwrap();
        let items = vec![ExportItem::Document { name: "../secret.md".into(), title: None }];
        assert!(load_parts(dir.path(), &items).is_err());
    }
}


