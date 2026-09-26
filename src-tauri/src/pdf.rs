//! PDF export: the compiled manuscript is turned into Typst markup and typeset
//! with the Typst engine, so the PDF has real page layout (justified text,
//! hyphenation, page numbers, chapter breaks) without a browser or print dialog.

use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};
use std::sync::OnceLock;
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

#[derive(Debug, Clone, Copy, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PageSize {
    #[default]
    A4,
    Letter,
}

pub struct PdfLayout<'a> {
    pub title: &'a str,
    pub author: &'a str,
    pub page_size: PageSize,
    /// Standard manuscript format: 12pt, double spaced, ragged right.
    pub manuscript: bool,
    pub chapter_page_breaks: bool,
    /// Validated ISO 639 code, e.g. "en".
    pub lang: &'a str,
    /// A contents page listing headings down to this level.
    pub toc_depth: Option<usize>,
    /// Text before the page number at the top of each page, e.g.
    /// "Writer / MY NOVEL" (may be empty). `None` keeps numbers at the bottom.
    pub running_head: Option<String>,
    /// Front matter pages, as Typst, placed before the contents.
    pub front: &'a str,
}

// ---------------------------------------------------------------------------
// Markdown events -> Typst markup
// ---------------------------------------------------------------------------

/// Escapes prose so nothing in it is read as Typst syntax. Quotes are left
/// alone so Typst turns them into typographic quotes.
pub(crate) fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        if matches!(
            c,
            '\\' | '#' | '*' | '_' | '`' | '$' | '<' | '>' | '@' | '[' | ']' | '(' | ')' | '{' | '}'
                | '~' | '/' | '=' | '-' | '+' | '.' | ':'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A Typst string literal.
fn string_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn heading_level(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Converts cleaned Markdown events into Typst markup. Nested constructs build
/// into their own buffer and are wrapped when they close.
pub fn events_to_typst(events: Vec<Event<'_>>) -> String {
    // Each open container gets a buffer; the bottom one is the document.
    let mut buffers: Vec<String> = vec![String::new()];
    // Cells of tables being built, and their column counts.
    let mut tables: Vec<(usize, Vec<String>)> = Vec::new();
    let mut code_block: Option<String> = None;
    let mut discarding = 0usize;

    macro_rules! out {
        () => {
            buffers.last_mut().expect("document buffer is never popped")
        };
    }

    for event in events {
        if discarding > 0 {
            match event {
                Event::Start(Tag::FootnoteDefinition(_)) => discarding += 1,
                Event::End(TagEnd::FootnoteDefinition) => discarding -= 1,
                _ => {}
            }
            continue;
        }
        if let Some(code) = code_block.as_mut() {
            match event {
                Event::Text(text) => code.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let code = code_block.take().unwrap_or_default();
                    out!().push_str(&format!(
                        "#raw(block: true, {})\n\n",
                        string_literal(code.trim_end_matches('\n'))
                    ));
                }
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading { level, .. } => {
                    out!().push_str(&format!("#heading(level: {})[", heading_level(level)));
                }
                Tag::BlockQuote(_) => {
                    out!().push_str("#quote(block: true)[");
                }
                Tag::CodeBlock(_) => code_block = Some(String::new()),
                Tag::List(start) => match start {
                    Some(n) => out!().push_str(&format!("#enum(start: {n}, ")),
                    None => out!().push_str("#list("),
                },
                Tag::Item => out!().push('['),
                Tag::Emphasis => out!().push_str("#emph["),
                Tag::Strong => out!().push_str("#strong["),
                Tag::Strikethrough => out!().push_str("#strike["),
                Tag::Link { dest_url, .. } => {
                    out!().push_str(&format!("#link({})[", string_literal(&dest_url)));
                }
                Tag::Table(alignments) => tables.push((alignments.len().max(1), Vec::new())),
                Tag::TableCell => buffers.push(String::new()),
                Tag::FootnoteDefinition(_) => discarding = 1,
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => out!().push_str("\n\n"),
                TagEnd::Heading(_) => out!().push_str("]\n\n"),
                TagEnd::BlockQuote(_) => {
                    let buf = out!();
                    buf.truncate(buf.trim_end().len());
                    buf.push_str("]\n\n");
                }
                TagEnd::List(_) => out!().push_str(")\n\n"),
                TagEnd::Item => {
                    let buf = out!();
                    buf.truncate(buf.trim_end().len());
                    buf.push_str("], ");
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => out!().push(']'),
                TagEnd::TableCell => {
                    let cell = buffers.pop().unwrap_or_default();
                    if let Some((_, cells)) = tables.last_mut() {
                        cells.push(format!("[{}]", cell.trim()));
                    }
                }
                TagEnd::Table => {
                    if let Some((columns, cells)) = tables.pop() {
                        out!().push_str(&format!("#table(columns: {columns}, {})\n\n", cells.join(", ")));
                    }
                }
                _ => {}
            },
            Event::Text(text) => out!().push_str(&escape_text(&text)),
            Event::Code(code) => out!().push_str(&format!("#raw({})", string_literal(&code))),
            Event::Html(raw) | Event::InlineHtml(raw) => {
                // The only HTML that survives cleaning is the scene separator.
                if let Some(inner) = raw
                    .trim()
                    .strip_prefix("<p class=\"separator\">")
                    .and_then(|s| s.strip_suffix("</p>"))
                {
                    let text = inner
                        .replace("&quot;", "\"")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&amp;", "&");
                    out!().push_str(&format!("#separator[{}]\n\n", escape_text(&text)));
                }
            }
            Event::SoftBreak => out!().push(' '),
            Event::HardBreak => out!().push_str("#linebreak()"),
            Event::Rule => out!().push_str("#separator[\\*~\\*~\\*]\n\n"),
            _ => {}
        }
    }

    buffers.into_iter().next().unwrap_or_default()
}

/// Page setup, fonts and heading styles, followed by the title page.
fn preamble(layout: &PdfLayout) -> String {
    let paper = match layout.page_size {
        PageSize::A4 => "a4",
        PageSize::Letter => "us-letter",
    };
    let (font, size, leading, spacing, justify) = if layout.manuscript {
        (r#"("Times New Roman", "Libertinus Serif")"#, "12pt", "1.5em", "1.5em", "false")
    } else {
        (r#"("Libertinus Serif", "New Computer Modern")"#, "11pt", "0.7em", "0.7em", "true")
    };
    let chapter_break = if layout.chapter_page_breaks { "pagebreak(weak: true)" } else { "v(2em)" };

    let mut out = format!(
        r#"#set document(title: {title}, author: {author})
#set page(paper: "{paper}", margin: (x: 2.5cm, y: 2.5cm), numbering: "1", number-align: center)
#set text(font: {font}, size: {size}, lang: {lang}, hyphenate: {hyphenate})
#set par(justify: {justify}, leading: {leading}, spacing: {spacing}, first-line-indent: (amount: 1.5em, all: false))
#show raw: set text(font: "DejaVu Sans Mono", size: 0.85em)
#let breathe(body) = {{ v(0.6em); body; v(0.6em) }}
#show quote.where(block: true): it => pad(x: 2em, text(style: "italic", it.body))
#show list: breathe
#show enum: breathe
#show table: breathe
#show raw.where(block: true): breathe
#show heading: it => block(above: 2em, below: 1em, sticky: true, text(weight: "regular", size: 1.2em, it.body))
#show heading.where(level: 1): it => {{
  {chapter_break}
  v(12%)
  align(center, text(size: 1.9em, weight: "regular", it.body))
  v(2.5em)
}}
#let separator(body) = block(width: 100%, inset: (y: 0.6em), align(center, body))
"#,
        title = string_literal(layout.title),
        author = string_literal(layout.author),
        lang = string_literal(layout.lang),
        hyphenate = !layout.manuscript,
    );

    if !layout.title.trim().is_empty() {
        out.push_str(&format!(
            "#page(numbering: none)[#v(30%)#align(center)[#text(size: 3.2em)[{}]",
            escape_text(layout.title.trim())
        ));
        if !layout.author.trim().is_empty() {
            out.push_str(&format!(
                "#v(1.5em)#text(size: 1.3em, style: \"italic\")[{}]",
                escape_text(layout.author.trim())
            ));
        }
        out.push_str("]]\n#counter(page).update(1)\n\n");
    }
    if let Some(head) = &layout.running_head {
        let prefix = if head.is_empty() { String::new() } else { format!("{} / ", escape_text(head)) };
        out.push_str(&format!(
            "#set page(header: align(right, text(size: 0.9em)[{prefix}#context counter(page).display()]), footer: none)\n"
        ));
    }
    out.push_str(layout.front);
    if let Some(depth) = layout.toc_depth {
        out.push_str(&format!(
            "#page[#align(center, text(size: 1.6em)[Contents])#v(1.5em)#outline(title: none, depth: {depth})]\n\n"
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Typesetting
// ---------------------------------------------------------------------------

/// Bundled fonts first (so output looks the same everywhere), then the
/// system's, for scripts the bundled fonts don't cover and for Times New Roman.
/// Scanning is slow, so it happens once per run.
fn fonts() -> &'static FontStore {
    static FONTS: OnceLock<FontStore> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut store = FontStore::new();
        store.extend(typst_kit::fonts::embedded());
        store.extend(typst_kit::fonts::system());
        store
    })
}

fn library() -> &'static LazyHash<Library> {
    static LIBRARY: OnceLock<LazyHash<Library>> = OnceLock::new();
    LIBRARY.get_or_init(|| LazyHash::new(Library::default()))
}

/// A world with a single in-memory source file and no file access.
struct ManuscriptWorld {
    source: Source,
}

impl World for ManuscriptWorld {
    fn library(&self) -> &LazyHash<Library> {
        library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        fonts().book()
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        fonts().font(index)
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        None
    }
}

fn describe(diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| d.message.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Typesets Typst markup (body only; the preamble is added here) into PDF bytes.
pub fn typeset(layout: &PdfLayout, body: &str) -> Result<Vec<u8>, String> {
    let markup = format!("{}{}", preamble(layout), body);
    let world = ManuscriptWorld { source: Source::detached(markup) };
    let document: PagedDocument = typst::compile(&world)
        .output
        .map_err(|errors| format!("Could not lay out the PDF: {}", describe(&errors)))?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|errors| format!("Could not write the PDF: {}", describe(&errors)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    fn convert(markdown: &str) -> String {
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
        events_to_typst(Parser::new_ext(markdown, options).collect())
    }

    fn layout() -> PdfLayout<'static> {
        PdfLayout { title: "My Novel", author: "A. Writer", page_size: PageSize::A4, manuscript: false, chapter_page_breaks: true, lang: "en", toc_depth: None, running_head: None, front: "" }
    }

    #[test]
    fn escapes_everything_typst_would_read_as_syntax() {
        let out = convert("Price: $5 #1 @me = 1. <b> (a)[b] ~/ 2-3");
        assert!(!out.contains(" $5"), "{out}");
        assert!(out.contains("\\$5") && out.contains("\\#1") && out.contains("\\@me") && out.contains("\\(a\\)"));
    }

    #[test]
    fn structure_maps_to_typst_functions() {
        let out = convert("# Title\n\nSome *em* and **strong**.\n\n- one\n- two\n\n> quoted\n\n```\nlet x = \"y\";\n```\n");
        assert!(out.contains("#heading(level: 1)[Title]"), "{out}");
        assert!(out.contains("#emph[em]") && out.contains("#strong[strong]"), "{out}");
        assert!(out.contains("#list([one], [two], )"), "{out}");
        assert!(out.contains("#quote(block: true)[quoted]"), "{out}");
        assert!(out.contains(r#"#raw(block: true, "let x = \"y\";")"#), "{out}");
    }

    #[test]
    fn hostile_text_still_compiles() {
        let body = convert(
            "# Chapter (One) [draft]\n\nHe said \"hi\" — *then* left.x(1) #set page() $$ `a\"b` \\\\ end.\n\n1. first\n2. second\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nFootnote[^1].\n\n[^1]: Gone.\n\n---\n\n日本語のテキスト。",
        );
        let pdf = typeset(&layout(), &body).expect("compiles");
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn manuscript_and_letter_layouts_compile() {
        let layout = PdfLayout { page_size: PageSize::Letter, manuscript: true, chapter_page_breaks: false, title: "", author: "", lang: "de", toc_depth: None, running_head: None, front: "" };
        let pdf = typeset(&layout, &convert("Plain text.")).expect("compiles");
        assert!(pdf.starts_with(b"%PDF"));
    }
}
