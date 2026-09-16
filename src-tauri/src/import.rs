//! Brings existing writing into the content directory: Word documents,
//! Scrivener projects, RTF, and folders of Markdown (including Obsidian vaults).
//!
//! Imports only ever create new files and folders; nothing existing is
//! overwritten. The app orders writings by name, so when a source has its own
//! order (Scrivener's binder, a document's headings) and the names wouldn't
//! sort that way, they get a "01 ", "02 " prefix.

use crate::rtf::rtf_to_markdown;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Content-relative names of the writings created.
    pub created: Vec<String>,
    /// Top-level folders created (one per imported project or folder).
    pub folders: Vec<String>,
    /// Things that were skipped or could not be converted, for the writer to see.
    pub warnings: Vec<String>,
}

/// Scene metadata stored as front matter at the top of a writing.
#[derive(Debug, Default, Clone)]
pub struct Meta {
    pub synopsis: Option<String>,
    pub status: Option<String>,
    pub label: Option<String>,
}

impl Meta {
    fn is_empty(&self) -> bool {
        self.synopsis.is_none() && self.status.is_none() && self.label.is_none()
    }
}

/// Values are JSON strings, which are also valid YAML, so any text round-trips.
pub fn with_front_matter(meta: &Meta, body: &str) -> String {
    if meta.is_empty() {
        return body.to_string();
    }
    let mut out = String::from("---\n");
    for (key, value) in [("synopsis", &meta.synopsis), ("status", &meta.status), ("label", &meta.label)] {
        if let Some(value) = value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            out.push_str(&format!("{key}: {}\n", serde_json::to_string(value).unwrap_or_default()));
        }
    }
    out.push_str("---\n\n");
    out.push_str(body);
    out
}

// ---------------------------------------------------------------------------
// Names and ordering
// ---------------------------------------------------------------------------

/// A file or folder name that is safe on every platform.
fn clean_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = cleaned.trim_matches('.').trim();
    let short: String = cleaned.chars().take(120).collect();
    if short.is_empty() { "Untitled".to_string() } else { short }
}

/// `base` + `ext` under `dir`, adding " 2", " 3"… if taken (case-insensitively,
/// since macOS and Windows file systems usually are).
fn unique_child(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let taken = |candidate: &str| -> bool {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries.flatten().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(candidate))
            })
            .unwrap_or(false)
    };
    let first = format!("{base}{ext}");
    if !taken(&first) {
        return dir.join(first);
    }
    (2..)
        .map(|n| format!("{base} {n}{ext}"))
        .find(|candidate| !taken(candidate))
        .map(|candidate| dir.join(candidate))
        .expect("unbounded range always finds a free name")
}

/// "Chapter 2" before "Chapter 10", ignoring case: the order the app's tree uses.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let na: String = std::iter::from_fn(|| a.next_if(|c| c.is_ascii_digit())).collect();
                let nb: String = std::iter::from_fn(|| b.next_if(|c| c.is_ascii_digit())).collect();
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(x), Some(y)) => {
                let ord = x.to_lowercase().cmp(y.to_lowercase());
                if ord != Ordering::Equal {
                    return ord;
                }
                a.next();
                b.next();
            }
        }
    }
}

/// Prefixes names with "01 ", "02 "… only when sorting them would change their order.
fn ordered_names(names: &[String]) -> Vec<String> {
    let mut sorted = names.to_vec();
    sorted.sort_by(|a, b| natural_cmp(a, b));
    if sorted == names {
        return names.to_vec();
    }
    let width = names.len().to_string().len().max(2);
    names
        .iter()
        .enumerate()
        .map(|(i, name)| format!("{:0width$} {name}", i + 1))
        .collect()
}

fn relative(content_dir: &Path, path: &Path) -> String {
    path.strip_prefix(content_dir)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn write_new(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    std::fs::write(path, content).map_err(|e| format!("Could not write {}: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// DOCX
// ---------------------------------------------------------------------------

/// A block of a Word document: a heading (with level) or body Markdown.
#[derive(Debug, PartialEq)]
enum DocBlock {
    Heading(u8, String),
    Body(String),
}

fn attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == name)
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

/// `<w:b/>` is on; `<w:b w:val="0"/>` or "false" is off.
fn toggle_on(e: &BytesStart) -> bool {
    !matches!(attr(e, b"val").as_deref(), Some("0" | "false" | "none"))
}

/// Heading level from a paragraph style id like "Heading1", "heading 2", "Titre1".
fn heading_from_style(style: &str) -> Option<u8> {
    let lower = style.to_lowercase();
    if lower == "title" {
        return Some(1);
    }
    let digits: String = lower.chars().filter(|c| c.is_ascii_digit()).collect();
    let looks_like_heading = ["heading", "berschrift", "titre", "titolo", "título", "kop"]
        .iter()
        .any(|word| lower.contains(word));
    if looks_like_heading {
        digits.parse::<u8>().ok().filter(|n| (1..=6).contains(n))
    } else {
        None
    }
}

fn parse_docx_blocks(document_xml: &str) -> Result<Vec<DocBlock>, String> {
    let mut reader = Reader::from_str(document_xml);
    let mut blocks = Vec::new();

    let mut in_paragraph = false;
    let mut heading: Option<u8> = None;
    let mut list_item = false;
    let mut runs: Vec<(String, bool, bool)> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut in_run_props = false;
    // Tab stops in paragraph properties are layout, not text.
    let mut in_para_props = false;
    let mut in_text = false;
    let mut in_deleted = 0usize;

    loop {
        let event = reader.read_event().map_err(|e| format!("Unreadable Word document: {e}"))?;
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(event, Event::Empty(_));
                match e.local_name().as_ref() {
                    b"p" if !empty => {
                        in_paragraph = true;
                        heading = None;
                        list_item = false;
                        runs.clear();
                    }
                    b"pStyle" => heading = attr(e, b"val").and_then(|s| heading_from_style(&s)).or(heading),
                    b"outlineLvl" => {
                        if let Some(level) = attr(e, b"val").and_then(|v| v.parse::<u8>().ok()) {
                            if level < 6 {
                                heading = Some(level + 1);
                            }
                        }
                    }
                    b"numPr" => list_item = true,
                    b"r" if !empty => {
                        bold = false;
                        italic = false;
                    }
                    b"rPr" if !empty => in_run_props = true,
                    b"pPr" if !empty => in_para_props = true,
                    b"b" if in_run_props => bold = toggle_on(e),
                    b"i" if in_run_props => italic = toggle_on(e),
                    b"t" if !empty => in_text = true,
                    b"tab" if in_paragraph && !in_para_props => runs.push(("\t".into(), false, false)),
                    b"br" | b"cr" if in_paragraph => runs.push(("\n".into(), false, false)),
                    b"del" if !empty => in_deleted += 1,
                    _ => {}
                }
            }
            Event::End(ref e) => match e.local_name().as_ref() {
                b"t" => in_text = false,
                b"rPr" => in_run_props = false,
                b"pPr" => in_para_props = false,
                b"del" => in_deleted = in_deleted.saturating_sub(1),
                b"p" if in_paragraph => {
                    in_paragraph = false;
                    let text: String = runs.iter().map(|(t, _, _)| t.as_str()).collect();
                    if text.trim().is_empty() {
                        continue;
                    }
                    match heading {
                        Some(level) => blocks.push(DocBlock::Heading(level, text.trim().to_string())),
                        None => {
                            let mut line = String::new();
                            for (text, bold, italic) in merge_runs(std::mem::take(&mut runs)) {
                                line.push_str(&render_docx_run(&text, bold, italic));
                            }
                            let line = line.replace('\n', "  \n");
                            let line = line.trim();
                            blocks.push(DocBlock::Body(if list_item { format!("- {line}") } else { line.to_string() }));
                        }
                    }
                }
                _ => {}
            },
            Event::Text(ref t) if in_text && in_deleted == 0 => {
                let text = t.decode().map_err(|e| e.to_string())?;
                push_run(&mut runs, &text, bold, italic);
            }
            Event::GeneralRef(ref r) if in_text && in_deleted == 0 => {
                let name = r.decode().map_err(|e| e.to_string())?;
                if let Ok(text) = quick_xml::escape::unescape(&format!("&{name};")) {
                    push_run(&mut runs, &text, bold, italic);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(blocks)
}

fn push_run(runs: &mut Vec<(String, bool, bool)>, text: &str, bold: bool, italic: bool) {
    runs.push((text.to_string(), bold, italic));
}

fn merge_runs(runs: Vec<(String, bool, bool)>) -> Vec<(String, bool, bool)> {
    let mut merged: Vec<(String, bool, bool)> = Vec::new();
    for (text, bold, italic) in runs {
        match merged.last_mut() {
            Some(last) if last.1 == bold && last.2 == italic => last.0.push_str(&text),
            _ => merged.push((text, bold, italic)),
        }
    }
    merged
}

fn render_docx_run(text: &str, bold: bool, italic: bool) -> String {
    let escaped: String = text
        .chars()
        .flat_map(|c| {
            let escape = matches!(c, '\\' | '*' | '_' | '[' | ']' | '`' | '<');
            escape.then_some('\\').into_iter().chain(std::iter::once(c))
        })
        .collect();
    if !bold && !italic {
        return escaped;
    }
    let core = escaped.trim();
    if core.is_empty() {
        return escaped;
    }
    let lead = &escaped[..escaped.len() - escaped.trim_start().len()];
    let trail = &escaped[escaped.trim_end().len()..];
    let marker = match (bold, italic) {
        (true, true) => "***",
        (true, false) => "**",
        _ => "*",
    };
    format!("{lead}{marker}{core}{marker}{trail}")
}

fn read_docx_blocks(path: &Path) -> Result<Vec<DocBlock>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| format!("{} is not a Word document", path.display()))?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|_| format!("{} is not a Word document", path.display()))?
        .read_to_string(&mut xml)
        .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    parse_docx_blocks(&xml)
}

fn blocks_to_markdown(blocks: &[DocBlock]) -> String {
    let mut out = Vec::new();
    for block in blocks {
        match block {
            DocBlock::Heading(level, text) => out.push(format!("{} {text}", "#".repeat(*level as usize))),
            DocBlock::Body(text) => out.push(text.clone()),
        }
    }
    // Consecutive list items belong to one list.
    let mut joined = String::new();
    for (i, block) in out.iter().enumerate() {
        if i > 0 {
            let tight = block.starts_with("- ") && out[i - 1].starts_with("- ");
            joined.push_str(if tight { "\n" } else { "\n\n" });
        }
        joined.push_str(block);
    }
    joined.push('\n');
    joined
}

/// A Word document becomes one writing, or with `split` a folder holding one
/// writing per top-level heading.
fn import_docx(path: &Path, target: &Path, content_dir: &Path, split: bool, report: &mut ImportReport) -> Result<(), String> {
    let blocks = read_docx_blocks(path)?;
    let stem = clean_name(&path.file_stem().unwrap_or_default().to_string_lossy());

    let top = blocks
        .iter()
        .filter_map(|b| match b {
            DocBlock::Heading(level, _) => Some(*level),
            _ => None,
        })
        .min();

    let Some(top) = top.filter(|_| split) else {
        let file = unique_child(target, &stem, ".md");
        write_new(&file, &blocks_to_markdown(&blocks))?;
        report.created.push(relative(content_dir, &file));
        return Ok(());
    };

    let mut sections: Vec<(String, Vec<&DocBlock>)> = Vec::new();
    for block in &blocks {
        match block {
            DocBlock::Heading(level, text) if *level == top => sections.push((clean_name(text), Vec::new())),
            _ => {
                if sections.is_empty() {
                    sections.push(("Opening".to_string(), Vec::new()));
                }
                sections.last_mut().expect("just ensured").1.push(block);
            }
        }
    }

    let folder = unique_child(target, &stem, "");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    report.folders.push(relative(content_dir, &folder));
    let names = ordered_names(&sections.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>());
    for ((_, members), name) in sections.iter().zip(names) {
        // The heading became the file name; deeper headings stay in the text.
        let owned: Vec<DocBlock> = members
            .iter()
            .map(|b| match b {
                DocBlock::Heading(l, t) => DocBlock::Heading((*l).saturating_sub(top).max(1), t.clone()),
                DocBlock::Body(t) => DocBlock::Body(t.clone()),
            })
            .collect();
        let file = unique_child(&folder, &name, ".md");
        write_new(&file, &blocks_to_markdown(&owned))?;
        report.created.push(relative(content_dir, &file));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scrivener
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct BinderItem {
    id: String,
    kind: String,
    title: String,
    status_id: Option<String>,
    label_id: Option<String>,
    include_in_compile: bool,
    children: Vec<BinderItem>,
}

#[derive(Debug, Default)]
struct ScrivenerProject {
    items: Vec<BinderItem>,
    statuses: HashMap<String, String>,
    labels: HashMap<String, String>,
}

fn parse_scrivx(xml: &str) -> Result<ScrivenerProject, String> {
    let mut reader = Reader::from_str(xml);
    let mut project = ScrivenerProject::default();
    let mut stack: Vec<BinderItem> = Vec::new();
    // Name of the element whose text we're collecting, with the id for status/label entries.
    let mut capture: Option<(Vec<u8>, Option<String>)> = None;
    let mut text = String::new();

    loop {
        match reader.read_event().map_err(|e| format!("Unreadable Scrivener project: {e}"))? {
            Event::Start(e) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"BinderItem" => stack.push(BinderItem {
                        id: attr(&e, b"UUID").or_else(|| attr(&e, b"ID")).unwrap_or_default(),
                        kind: attr(&e, b"Type").unwrap_or_default(),
                        include_in_compile: true,
                        ..Default::default()
                    }),
                    b"Title" | b"StatusID" | b"LabelID" | b"IncludeInCompile" => {
                        capture = Some((name, None));
                        text.clear();
                    }
                    b"Status" | b"Label" => {
                        capture = Some((name, attr(&e, b"ID")));
                        text.clear();
                    }
                    _ => {}
                }
            }
            Event::Text(t) => {
                if capture.is_some() {
                    text.push_str(&t.decode().map_err(|e| e.to_string())?);
                }
            }
            Event::GeneralRef(r) => {
                if capture.is_some() {
                    let name = r.decode().map_err(|e| e.to_string())?;
                    if let Ok(t) = quick_xml::escape::unescape(&format!("&{name};")) {
                        text.push_str(&t);
                    }
                }
            }
            Event::End(e) => {
                let name = e.local_name().as_ref().to_vec();
                if let Some((captured, id)) = capture.take_if(|(n, _)| *n == name) {
                    let value = text.trim().to_string();
                    match (captured.as_slice(), stack.last_mut()) {
                        (b"Title", Some(item)) => item.title = value,
                        (b"StatusID", Some(item)) => item.status_id = Some(value),
                        (b"LabelID", Some(item)) => item.label_id = Some(value),
                        (b"IncludeInCompile", Some(item)) => item.include_in_compile = value.eq_ignore_ascii_case("yes"),
                        (b"Status", _) => {
                            if let Some(id) = id {
                                project.statuses.insert(id, value);
                            }
                        }
                        (b"Label", _) => {
                            if let Some(id) = id {
                                project.labels.insert(id, value);
                            }
                        }
                        _ => {}
                    }
                }
                if name == b"BinderItem" {
                    if let Some(item) = stack.pop() {
                        match stack.last_mut() {
                            Some(parent) => parent.children.push(item),
                            None => project.items.push(item),
                        }
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(project)
}

/// Where Scrivener keeps an item's text and synopsis (3.x, then 2.x layout).
fn scrivener_files(scriv: &Path, id: &str) -> (Option<PathBuf>, Option<PathBuf>) {
    let v3 = scriv.join("Files").join("Data").join(id);
    let v2 = scriv.join("Files").join("Docs");
    let content = [v3.join("content.rtf"), v2.join(format!("{id}.rtf"))]
        .into_iter()
        .find(|p| p.exists());
    let synopsis = [v3.join("synopsis.txt"), v2.join(format!("{id}_synopsis.txt"))]
        .into_iter()
        .find(|p| p.exists());
    (content, synopsis)
}

struct ScrivenerImport<'a> {
    scriv: &'a Path,
    content_dir: &'a Path,
    project: &'a ScrivenerProject,
    skipped_media: usize,
}

impl ScrivenerImport<'_> {
    fn meta_for(&self, item: &BinderItem) -> Result<Meta, String> {
        let (_, synopsis_path) = scrivener_files(self.scriv, &item.id);
        let synopsis = match synopsis_path {
            Some(p) => Some(std::fs::read_to_string(&p).map_err(|e| format!("Could not read {}: {e}", p.display()))?),
            None => None,
        };
        let lookup = |id: &Option<String>, table: &HashMap<String, String>| {
            id.as_ref()
                .filter(|id| id.as_str() != "-1")
                .and_then(|id| table.get(id))
                .filter(|name| !matches!(name.as_str(), "No Status" | "No Label"))
                .cloned()
        };
        Ok(Meta {
            synopsis: synopsis.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            status: lookup(&item.status_id, &self.project.statuses),
            label: lookup(&item.label_id, &self.project.labels),
        })
    }

    fn write_items(&mut self, items: &[&BinderItem], dir: &Path, report: &mut ImportReport) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let titles: Vec<String> = items.iter().map(|i| clean_name(&i.title)).collect();
        for (item, name) in items.iter().zip(ordered_names(&titles)) {
            let is_folder = item.kind.ends_with("Folder");
            let is_text = item.kind == "Text";
            if !is_folder && !is_text {
                self.skipped_media += 1;
                continue;
            }

            let (content_path, _) = scrivener_files(self.scriv, &item.id);
            let body = match content_path {
                Some(p) => {
                    let raw = std::fs::read(&p).map_err(|e| format!("Could not read {}: {e}", p.display()))?;
                    rtf_to_markdown(&String::from_utf8_lossy(&raw))
                }
                None => String::new(),
            };
            let meta = self.meta_for(item)?;

            // Folders with their own text (or text items with children) keep the
            // text as a writing next to the sub-folder of the same name.
            if is_text || !body.trim().is_empty() || !meta.is_empty() {
                let file = unique_child(dir, &name, ".md");
                let mut content = with_front_matter(&meta, &body);
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                write_new(&file, &content)?;
                report.created.push(relative(self.content_dir, &file));
            }
            if is_folder || !item.children.is_empty() {
                let children: Vec<&BinderItem> = item.children.iter().collect();
                let sub = if is_folder { unique_child(dir, &name, "") } else { dir.join(&name) };
                self.write_items(&children, &sub, report)?;
            }
        }
        Ok(())
    }
}

fn import_scrivener(scriv: &Path, target: &Path, content_dir: &Path, report: &mut ImportReport) -> Result<(), String> {
    let scrivx = std::fs::read_dir(scriv)
        .map_err(|e| format!("Could not open {}: {e}", scriv.display()))?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("scrivx")))
        .ok_or_else(|| format!("{} has no .scrivx file; is it a Scrivener project?", scriv.display()))?;
    let xml = std::fs::read_to_string(&scrivx).map_err(|e| format!("Could not read {}: {e}", scrivx.display()))?;
    let project = parse_scrivx(&xml)?;

    let stem = clean_name(&scriv.file_stem().unwrap_or_default().to_string_lossy());
    let root = unique_child(target, &stem, "");
    report.folders.push(relative(content_dir, &root));

    let items: Vec<&BinderItem> = project.items.iter().filter(|i| i.kind != "TrashFolder").collect();
    let excluded = count_excluded(&project.items);
    let mut import = ScrivenerImport { scriv, content_dir, project: &project, skipped_media: 0 };
    import.write_items(&items, &root, report)?;

    if import.skipped_media > 0 {
        report.warnings.push(format!(
            "{}: skipped {} PDFs, images or web pages (only text is imported).",
            stem, import.skipped_media
        ));
    }
    if excluded > 0 {
        report.warnings.push(format!(
            "{stem}: {excluded} items were marked “not included in compile” in Scrivener; they were imported anyway."
        ));
    }
    Ok(())
}

fn count_excluded(items: &[BinderItem]) -> usize {
    items
        .iter()
        .filter(|i| i.kind != "TrashFolder")
        .map(|i| usize::from(!i.include_in_compile && i.kind == "Text") + count_excluded(&i.children))
        .sum()
}

// ---------------------------------------------------------------------------
// Markdown folders and single files
// ---------------------------------------------------------------------------

const TEXT_EXTENSIONS: &[&str] = &["md", "markdown", "txt"];

fn has_extension(path: &Path, list: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| list.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Same encoding as the editor's `encodeURIComponent`, plus parentheses so the
/// link destination never ends early.
fn encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Turns Obsidian `[[Note]]`, `[[Note|Alias]]` and `[[Note#Heading]]` into links
/// between writings; embeds (`![[...]]`) and unknown targets become plain text.
fn convert_wikilinks(markdown: &str, lookup: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(start) = rest.find("[[") {
        let Some(len) = rest[start + 2..].find("]]") else { break };
        let inner = &rest[start + 2..start + 2 + len];
        if inner.contains('\n') || inner.contains("[[") {
            out.push_str(&rest[..start + 2]);
            rest = &rest[start + 2..];
            continue;
        }
        let embed = start > 0 && rest.as_bytes()[start - 1] == b'!';
        out.push_str(&rest[..if embed { start - 1 } else { start }]);

        let (target, alias) = match inner.split_once('|') {
            Some((t, a)) => (t, Some(a)),
            None => (inner, None),
        };
        let note = target.split('#').next().unwrap_or(target).trim();
        let text = alias.unwrap_or(target).trim();
        let key = note.rsplit('/').next().unwrap_or(note).to_lowercase();
        match lookup.get(&key).filter(|_| !embed) {
            Some(name) => out.push_str(&format!(
                "[{}](#writing/{})",
                text.replace('[', "\\[").replace(']', "\\]"),
                encode_component(name)
            )),
            None => out.push_str(text),
        }
        rest = &rest[start + 2 + len + 2..];
    }
    out.push_str(rest);
    out
}

fn collect_text_files(dir: &Path, found: &mut Vec<PathBuf>, skipped: &mut usize) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("Could not open {}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        let hidden = path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if hidden {
            continue;
        }
        if path.is_dir() {
            collect_text_files(&path, found, skipped)?;
        } else if has_extension(&path, TEXT_EXTENSIONS) {
            found.push(path);
        } else {
            *skipped += 1;
        }
    }
    Ok(())
}

fn import_markdown_folder(source: &Path, target: &Path, content_dir: &Path, report: &mut ImportReport) -> Result<(), String> {
    let mut files = Vec::new();
    let mut skipped = 0;
    collect_text_files(source, &mut files, &mut skipped)?;
    let stem = clean_name(&source.file_name().unwrap_or_default().to_string_lossy());
    if files.is_empty() {
        report.warnings.push(format!("{stem}: no Markdown or text files found."));
        return Ok(());
    }

    let root = unique_child(target, &stem, "");
    report.folders.push(relative(content_dir, &root));

    // Plan every destination first so links can point at the final names.
    let mut plan: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut lookup: HashMap<String, String> = HashMap::new();
    for file in &files {
        let rel = file.strip_prefix(source).unwrap_or(file);
        let mut dest_dir = root.clone();
        if let Some(parent) = rel.parent() {
            for part in parent.components() {
                dest_dir.push(clean_name(&part.as_os_str().to_string_lossy()));
            }
        }
        std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
        let base = clean_name(&file.file_stem().unwrap_or_default().to_string_lossy());
        let mut dest = unique_child(&dest_dir, &base, ".md");
        // unique_child only sees the disk; also avoid names planned in this run.
        let mut n = 2;
        while plan.iter().any(|(_, d)| d.to_string_lossy().eq_ignore_ascii_case(&dest.to_string_lossy())) {
            dest = dest_dir.join(format!("{base} {n}.md"));
            n += 1;
        }
        lookup
            .entry(file.file_stem().unwrap_or_default().to_string_lossy().to_lowercase())
            .or_insert_with(|| relative(content_dir, &dest));
        plan.push((file.clone(), dest));
    }

    for (source_file, dest) in plan {
        let raw = std::fs::read(&source_file).map_err(|e| format!("Could not read {}: {e}", source_file.display()))?;
        let text = String::from_utf8_lossy(&raw);
        write_new(&dest, &convert_wikilinks(&text, &lookup))?;
        report.created.push(relative(content_dir, &dest));
    }
    if skipped > 0 {
        report.warnings.push(format!("{stem}: skipped {skipped} files that aren't Markdown or text (images, PDFs…)."));
    }
    Ok(())
}

fn import_single_text(path: &Path, target: &Path, content_dir: &Path, report: &mut ImportReport) -> Result<(), String> {
    let raw = std::fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let text = String::from_utf8_lossy(&raw);
    let body = if has_extension(path, &["rtf"]) { rtf_to_markdown(&text) } else { text.into_owned() };
    let stem = clean_name(&path.file_stem().unwrap_or_default().to_string_lossy());
    let file = unique_child(target, &stem, ".md");
    write_new(&file, &body)?;
    report.created.push(relative(content_dir, &file));
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Imports each source into `target_dir` (content-relative, "" for the top
/// level). A source that fails is reported as a warning; the rest still import.
pub fn import_sources(
    sources: &[PathBuf],
    target_dir: &str,
    split_docx: bool,
    content_dir: &Path,
) -> Result<ImportReport, String> {
    if Path::new(target_dir).components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return Err(format!("Invalid destination: {target_dir}"));
    }
    let target = content_dir.join(target_dir);
    std::fs::create_dir_all(&target).map_err(|e| format!("Could not create {}: {e}", target.display()))?;

    let mut report = ImportReport::default();
    for source in sources {
        // Refuse to import the library into itself.
        if content_dir.starts_with(source) || source.starts_with(content_dir) {
            report.warnings.push(format!("{} is inside your library already.", source.display()));
            continue;
        }
        let result = if source.is_dir() && has_extension(source, &["scriv"]) {
            import_scrivener(source, &target, content_dir, &mut report)
        } else if source.is_dir() {
            import_markdown_folder(source, &target, content_dir, &mut report)
        } else if has_extension(source, &["docx"]) {
            import_docx(source, &target, content_dir, split_docx, &mut report)
        } else if has_extension(source, &["md", "markdown", "txt", "rtf"]) {
            import_single_text(source, &target, content_dir, &mut report)
        } else {
            Err(format!("{} isn't a format Memorie can import.", source.display()))
        };
        if let Err(e) = result {
            report.warnings.push(e);
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn natural_order_and_prefixes() {
        assert_eq!(natural_cmp("Chapter 2", "chapter 10"), Ordering::Less);
        let sorted = vec!["Arrival".to_string(), "Departure".to_string()];
        assert_eq!(ordered_names(&sorted), sorted);
        let unsorted = vec!["Opening".to_string(), "Arrival".to_string()];
        assert_eq!(ordered_names(&unsorted), vec!["01 Opening", "02 Arrival"]);
    }

    #[test]
    fn front_matter_quotes_values() {
        let meta = Meta { synopsis: Some("She says: \"no\"\nThen leaves.".into()), status: Some("First Draft".into()), label: None };
        let out = with_front_matter(&meta, "Body");
        assert_eq!(out, "---\nsynopsis: \"She says: \\\"no\\\"\\nThen leaves.\"\nstatus: \"First Draft\"\n---\n\nBody");
        assert_eq!(with_front_matter(&Meta::default(), "Body"), "Body");
    }

    #[test]
    fn docx_headings_emphasis_lists_and_entities() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
            <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Chapter One</w:t></w:r></w:p>
            <w:p><w:r><w:t xml:space="preserve">It was </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>dark</w:t></w:r><w:r><w:t xml:space="preserve"> &amp; cold.</w:t></w:r></w:p>
            <w:p><w:r><w:rPr><w:i/><w:b w:val="0"/></w:rPr><w:t>quiet</w:t></w:r><w:del><w:r><w:delText>gone</w:delText></w:r></w:del></w:p>
            <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/></w:numPr></w:pPr><w:r><w:t>one</w:t></w:r></w:p>
            <w:p><w:pPr><w:numPr/></w:pPr><w:r><w:t>two</w:t></w:r></w:p>
            <w:p/>
        </w:body></w:document>"#;
        let blocks = parse_docx_blocks(xml).unwrap();
        assert_eq!(blocks[0], DocBlock::Heading(1, "Chapter One".into()));
        assert_eq!(blocks[1], DocBlock::Body("It was **dark** & cold.".into()));
        assert_eq!(blocks[2], DocBlock::Body("*quiet*".into()));
        assert_eq!(blocks_to_markdown(&blocks[3..]), "- one\n- two\n");
    }

    fn write_docx(path: &Path, document_xml: &str) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn docx_split_into_ordered_writings() {
        let content = tempdir().unwrap();
        let sources = tempdir().unwrap();
        let docx = sources.path().join("My Book.docx");
        let heading = |t: &str| format!(r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>{t}</w:t></w:r></w:p>"#);
        let para = |t: &str| format!(r#"<w:p><w:r><w:t>{t}</w:t></w:r></w:p>"#);
        let xml = format!(
            r#"<w:document xmlns:w="x"><w:body>{}{}{}{}{}</w:body></w:document>"#,
            para("Dedication."),
            heading("Winter"),
            para("Cold."),
            heading("Autumn"),
            para("Leaves.")
        );
        write_docx(&docx, &xml);

        let report = import_sources(&[docx], "", true, content.path()).unwrap();
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(report.created, vec!["My Book/01 Opening.md", "My Book/02 Winter.md", "My Book/03 Autumn.md"]);
        let winter = std::fs::read_to_string(content.path().join("My Book/02 Winter.md")).unwrap();
        assert_eq!(winter, "Cold.\n");
    }

    #[test]
    fn scrivener_project_with_metadata() {
        let content = tempdir().unwrap();
        let sources = tempdir().unwrap();
        let scriv = sources.path().join("Novel.scriv");
        let data = scriv.join("Files/Data");
        let scene_a = "AAAA-1";
        let scene_b = "BBBB-2";
        for (id, rtf) in [(scene_a, r"{\rtf1 The {\i end} of it.\par}"), (scene_b, r"{\rtf1 Before it all.\par}")] {
            std::fs::create_dir_all(data.join(id)).unwrap();
            std::fs::write(data.join(id).join("content.rtf"), rtf).unwrap();
        }
        std::fs::write(data.join(scene_a).join("synopsis.txt"), "Everything ends.").unwrap();
        std::fs::write(
            scriv.join("Novel.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject>
  <Binder>
    <BinderItem UUID="D" Type="DraftFolder"><Title>Draft</Title><Children>
      <BinderItem UUID="C1" Type="Folder"><Title>Chapter 1</Title><Children>
        <BinderItem UUID="{scene_a}" Type="Text"><Title>Zebra &amp; End</Title><MetaData><StatusID>2</StatusID><LabelID>-1</LabelID></MetaData></BinderItem>
        <BinderItem UUID="{scene_b}" Type="Text"><Title>Aardvark</Title></BinderItem>
      </Children></BinderItem>
    </Children></BinderItem>
    <BinderItem UUID="R" Type="ResearchFolder"><Title>Research</Title><Children>
      <BinderItem UUID="P" Type="PDF"><Title>Map</Title></BinderItem>
    </Children></BinderItem>
    <BinderItem UUID="T" Type="TrashFolder"><Title>Trash</Title></BinderItem>
  </Binder>
  <StatusSettings><StatusItems><Status ID="2">First Draft</Status></StatusItems></StatusSettings>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let report = import_sources(&[scriv], "", false, content.path()).unwrap();
        assert_eq!(report.created, vec!["Novel/Draft/Chapter 1/01 Zebra & End.md", "Novel/Draft/Chapter 1/02 Aardvark.md"]);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        let zebra = std::fs::read_to_string(content.path().join("Novel/Draft/Chapter 1/01 Zebra & End.md")).unwrap();
        assert_eq!(zebra, "---\nsynopsis: \"Everything ends.\"\nstatus: \"First Draft\"\n---\n\nThe *end* of it.\n");
        assert!(content.path().join("Novel/Research").is_dir());
        assert!(!content.path().join("Novel/Trash").exists());
    }

    #[test]
    fn obsidian_vault_links_and_hidden_folders() {
        let content = tempdir().unwrap();
        let sources = tempdir().unwrap();
        let vault = sources.path().join("Vault");
        std::fs::create_dir_all(vault.join(".obsidian")).unwrap();
        std::fs::create_dir_all(vault.join("People")).unwrap();
        std::fs::write(vault.join(".obsidian/app.json"), "{}").unwrap();
        std::fs::write(vault.join("People/Ann (Lead).md"), "Ann.").unwrap();
        std::fs::write(vault.join("Plot.md"), "See [[Ann (Lead)|Ann]], [[Missing]] and ![[map.png]].").unwrap();
        std::fs::write(vault.join("map.png"), [0u8; 4]).unwrap();

        let report = import_sources(&[vault], "Imports", false, content.path()).unwrap();
        assert_eq!(report.created.len(), 2);
        let plot = std::fs::read_to_string(content.path().join("Imports/Vault/Plot.md")).unwrap();
        assert_eq!(plot, "See [Ann](#writing/Imports%2FVault%2FPeople%2FAnn%20%28Lead%29.md), Missing and map.png.");
        assert_eq!(report.warnings.len(), 1);
        assert!(!content.path().join("Imports/Vault/.obsidian").exists());
    }

    #[test]
    fn never_overwrites_and_rejects_bad_targets() {
        let content = tempdir().unwrap();
        let sources = tempdir().unwrap();
        let note = sources.path().join("Note.md");
        std::fs::write(&note, "new").unwrap();
        std::fs::write(content.path().join("Note.md"), "old").unwrap();
        let report = import_sources(&[note.clone()], "", false, content.path()).unwrap();
        assert_eq!(report.created, vec!["Note 2.md"]);
        assert_eq!(std::fs::read_to_string(content.path().join("Note.md")).unwrap(), "old");
        assert!(import_sources(&[note], "../outside", false, content.path()).is_err());
    }
}
