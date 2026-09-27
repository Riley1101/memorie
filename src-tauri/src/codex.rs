//! The codex: characters, locations, factions and items a story keeps track of.
//!
//! Each entry is a markdown file under a `Codex/` folder, either inside a binder
//! (`Novel/Codex/Characters/Aria.md`) or at the top of the content directory
//! (`Codex/Locations/The Iron Citadel.md`) for entries every binder shares. The
//! sheet lives in YAML front matter and the body holds free notes:
//!
//! ```text
//! ---
//! codex: character
//! aliases: [Ari]
//! summary: Exiled heir who hunts the man who burned Veyra.
//! fields:
//!   appearance: Ash-grey eyes, burn scar on her left hand.
//! relationships:
//!   - { to: Kael, kind: rival }
//! ---
//! Free notes...
//! ```
//!
//! Keys the codex doesn't know are kept as they were. Being plain files, entries
//! sync, version and index like any other note; `fs` keeps them out of the
//! writing lists.

use crate::fs::{self, DEFAULT_EXTENSION};
use aho_corasick::{AhoCorasick, MatchKind};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Name of the folder that holds codex entries, in a binder or at the top level.
pub const CODEX_DIR: &str = "Codex";

/// Longest stretch of an entry's notes put in front of the model.
const CONTEXT_NOTES_CHARS: usize = 600;
/// Most entries put in front of the model for one reply.
const CONTEXT_MAX_ENTITIES: usize = 8;

/// True for a content-relative file name that sits in a codex folder, either the
/// shared one (`Codex/...`) or a binder's (`Novel/Codex/...`).
pub fn is_codex_name(name: &str) -> bool {
    let dirs: Vec<&str> = match name.rsplit_once('/') {
        Some((dir, _)) => dir.split('/').collect(),
        None => return false,
    };
    dirs.first() == Some(&CODEX_DIR) || dirs.get(1) == Some(&CODEX_DIR)
}

/// True for a content-relative folder path that is, or is inside, a codex folder.
pub fn is_codex_folder(path: &str) -> bool {
    let dirs: Vec<&str> = path.split('/').collect();
    dirs.first() == Some(&CODEX_DIR) || dirs.get(1) == Some(&CODEX_DIR)
}

/// Refuses a writing (`is_file`) or folder path that would land in a codex
/// folder, where it would vanish from the binder tree.
pub fn ensure_not_reserved(path: &str, is_file: bool) -> Result<(), String> {
    let reserved = if is_file { is_codex_name(path) } else { is_codex_folder(path) };
    if reserved {
        return Err(format!(
            "\"{CODEX_DIR}\" is reserved for the codex. Choose another name for this folder."
        ));
    }
    Ok(())
}

/// Refuses paths that could reach outside the content directory.
pub fn ensure_relative(path: &str) -> Result<(), String> {
    let bad = path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.split('/').any(|s| s.is_empty() || s == "." || s == "..");
    if bad {
        return Err(format!("\"{path}\" is not a valid path"));
    }
    Ok(())
}

/// A binder name: one path segment.
fn ensure_binder(binder: &str) -> Result<(), String> {
    ensure_relative(binder)?;
    if binder.contains('/') {
        return Err(format!("\"{binder}\" is not a binder"));
    }
    Ok(())
}

// -------------------------------------------------------
//  ENTRIES
// -------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Character,
    Location,
    Faction,
    Item,
}

impl EntityKind {
    fn as_str(self) -> &'static str {
        match self {
            EntityKind::Character => "character",
            EntityKind::Location => "location",
            EntityKind::Faction => "faction",
            EntityKind::Item => "item",
        }
    }

    /// The folder entries of this kind are filed under.
    fn folder(self) -> &'static str {
        match self {
            EntityKind::Character => "Characters",
            EntityKind::Location => "Locations",
            EntityKind::Faction => "Factions",
            EntityKind::Item => "Items",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "character" | "characters" => Some(EntityKind::Character),
            "location" | "locations" => Some(EntityKind::Location),
            "faction" | "factions" => Some(EntityKind::Faction),
            "item" | "items" => Some(EntityKind::Item),
            _ => None,
        }
    }
}

/// How an entry's name and aliases are found in text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum MatchMode {
    /// "Will" matches "Will" but not "will".
    #[default]
    CaseSensitive,
    Insensitive,
    /// Never found automatically; only used when pinned or linked.
    Off,
}

impl MatchMode {
    fn as_str(self) -> &'static str {
        match self {
            MatchMode::CaseSensitive => "case-sensitive",
            MatchMode::Insensitive => "insensitive",
            MatchMode::Off => "off",
        }
    }

    fn parse(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "insensitive" | "case-insensitive" => MatchMode::Insensitive,
            "off" | "none" | "manual" => MatchMode::Off,
            _ => MatchMode::CaseSensitive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub to: String,
    #[serde(default)]
    pub kind: String,
}

/// One named field on a sheet ("appearance", "arc", ...), in the order written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    /// Content-relative file name, e.g. `Novel/Codex/Characters/Aria.md`.
    pub path: String,
    /// The binder the entry belongs to, or `None` for the shared codex.
    pub scope: Option<String>,
    pub name: String,
    pub kind: EntityKind,
    pub aliases: Vec<String>,
    pub summary: String,
    pub fields: Vec<Field>,
    pub relationships: Vec<Relationship>,
    pub match_mode: MatchMode,
    /// Always given to the model when writing in this entry's binder.
    pub pinned: bool,
    pub notes: String,
}

/// An entry as the sheet editor sends it back.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityInput {
    pub scope: Option<String>,
    pub name: String,
    pub kind: EntityKind,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub fields: Vec<Field>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    #[serde(default)]
    pub match_mode: MatchMode,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub notes: String,
}

impl Entity {
    /// The name and every alias, trimmed, without blanks or repeats.
    fn terms(&self) -> Vec<&str> {
        let mut seen = HashSet::new();
        std::iter::once(self.name.as_str())
            .chain(self.aliases.iter().map(String::as_str))
            .map(str::trim)
            .filter(|term| term.chars().count() >= 2 && seen.insert(*term))
            .collect()
    }
}

const KNOWN_KEYS: [&str; 7] = ["codex", "aliases", "summary", "fields", "relationships", "match", "pinned"];

/// Splits a file into its front matter (unparsed) and body.
fn split_front_matter(content: &str) -> (Option<&str>, &str) {
    let rest = match content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n")) {
        Some(rest) => rest,
        None => return (None, content),
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            let body = &rest[offset + line.len()..];
            return (Some(&rest[..offset]), body);
        }
        offset += line.len();
    }
    (None, content)
}

fn front_matter_map(content: &str) -> (Mapping, &str) {
    let (yaml, body) = split_front_matter(content);
    let map = yaml
        .and_then(|yaml| serde_yaml::from_str::<Value>(yaml).ok())
        .and_then(|value| match value {
            Value::Mapping(map) => Some(map),
            _ => None,
        });
    match map {
        Some(map) => (map, body),
        // Unreadable front matter stays part of the body, so saving can't lose it.
        None => (Mapping::new(), content),
    }
}

fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn get<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(Value::String(key.to_string()))
}

/// Reads an entry from its file. `name` is its content-relative file name.
pub fn parse_entity(name: &str, content: &str) -> Entity {
    let (map, body) = front_matter_map(content);
    let segments: Vec<&str> = name.split('/').collect();
    let scope = match segments.first() {
        Some(&CODEX_DIR) => None,
        Some(binder) => Some(binder.to_string()),
        None => None,
    };
    // `Codex/<Kind>/Name.md` or `Binder/Codex/<Kind>/Name.md`.
    let folder_kind = segments
        .iter()
        .position(|s| *s == CODEX_DIR)
        .and_then(|i| segments.get(i + 1))
        .filter(|_| segments.len() > 2)
        .and_then(|s| EntityKind::parse(s));

    let kind = get(&map, "codex")
        .and_then(scalar_string)
        .and_then(|k| EntityKind::parse(&k))
        .or(folder_kind)
        .unwrap_or(EntityKind::Character);

    let aliases = match get(&map, "aliases") {
        Some(Value::Sequence(items)) => items.iter().filter_map(scalar_string).collect(),
        Some(value) => scalar_string(value)
            .map(|s| s.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect())
            .unwrap_or_default(),
        None => Vec::new(),
    };

    let fields = match get(&map, "fields") {
        Some(Value::Mapping(fields)) => fields
            .iter()
            .filter_map(|(k, v)| Some(Field { key: scalar_string(k)?, value: scalar_string(v).unwrap_or_default() }))
            .collect(),
        _ => Vec::new(),
    };

    let relationships = match get(&map, "relationships") {
        Some(Value::Sequence(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Mapping(rel) => Some(Relationship {
                    to: get(rel, "to").and_then(scalar_string)?,
                    kind: get(rel, "kind").and_then(scalar_string).unwrap_or_default(),
                }),
                other => scalar_string(other).map(|to| Relationship { to, kind: String::new() }),
            })
            .filter(|rel| !rel.to.trim().is_empty())
            .collect(),
        _ => Vec::new(),
    };

    let file_stem = segments
        .last()
        .map(|s| s.strip_suffix(".md").unwrap_or(s))
        .unwrap_or_default();

    Entity {
        path: name.to_string(),
        scope,
        name: file_stem.to_string(),
        kind,
        aliases,
        summary: get(&map, "summary").and_then(scalar_string).unwrap_or_default(),
        fields,
        relationships,
        match_mode: get(&map, "match").and_then(scalar_string).map(|m| MatchMode::parse(&m)).unwrap_or_default(),
        pinned: matches!(get(&map, "pinned"), Some(Value::Bool(true))),
        notes: body.trim_start_matches(['\r', '\n']).trim_end().to_string(),
    }
}

/// Writes an entry as file content, keeping front matter keys the codex doesn't
/// know from `existing` (the file's current content, if any).
pub fn render_entity(input: &EntityInput, existing: Option<&str>) -> String {
    let mut map = Mapping::new();
    let s = |v: &str| Value::String(v.to_string());

    map.insert(s("codex"), s(input.kind.as_str()));
    let aliases: Vec<Value> = input
        .aliases
        .iter()
        .map(|a| a.trim())
        .filter(|a| !a.is_empty())
        .map(s)
        .collect();
    if !aliases.is_empty() {
        map.insert(s("aliases"), Value::Sequence(aliases));
    }
    if !input.summary.trim().is_empty() {
        map.insert(s("summary"), s(input.summary.trim()));
    }
    let mut fields = Mapping::new();
    for field in &input.fields {
        let key = field.key.trim();
        if !key.is_empty() && !field.value.trim().is_empty() {
            fields.insert(s(key), s(field.value.trim()));
        }
    }
    if !fields.is_empty() {
        map.insert(s("fields"), Value::Mapping(fields));
    }
    let relationships: Vec<Value> = input
        .relationships
        .iter()
        .filter(|rel| !rel.to.trim().is_empty())
        .map(|rel| {
            let mut m = Mapping::new();
            m.insert(s("to"), s(rel.to.trim()));
            if !rel.kind.trim().is_empty() {
                m.insert(s("kind"), s(rel.kind.trim()));
            }
            Value::Mapping(m)
        })
        .collect();
    if !relationships.is_empty() {
        map.insert(s("relationships"), Value::Sequence(relationships));
    }
    if input.match_mode != MatchMode::CaseSensitive {
        map.insert(s("match"), s(input.match_mode.as_str()));
    }
    if input.pinned {
        map.insert(s("pinned"), Value::Bool(true));
    }

    if let Some(existing) = existing {
        let (old, _) = front_matter_map(existing);
        for (key, value) in old {
            let known = key.as_str().is_some_and(|k| KNOWN_KEYS.contains(&k));
            if !known {
                map.insert(key, value);
            }
        }
    }

    let yaml = serde_yaml::to_string(&Value::Mapping(map)).unwrap_or_default();
    let notes = input.notes.trim();
    if notes.is_empty() {
        format!("---\n{yaml}---\n")
    } else {
        format!("---\n{yaml}---\n\n{notes}\n")
    }
}

/// Turns a display name into a safe file stem.
fn file_stem(name: &str) -> Result<String, String> {
    let stem: String = name
        .trim()
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '-' } else { c })
        .collect();
    let stem = stem.trim().trim_start_matches('.').trim().to_string();
    if stem.is_empty() {
        return Err("A codex entry needs a name".to_string());
    }
    Ok(stem)
}

/// Where an entry with this scope, kind and name is filed.
pub fn entry_path(scope: Option<&str>, kind: EntityKind, name: &str) -> Result<String, String> {
    let stem = file_stem(name)?;
    if let Some(binder) = scope.map(str::trim).filter(|s| !s.is_empty()) {
        ensure_binder(binder)?;
    }
    let base = format!("{CODEX_DIR}/{}/{stem}.{DEFAULT_EXTENSION}", kind.folder());
    Ok(match scope.map(str::trim).filter(|s| !s.is_empty()) {
        Some(binder) => format!("{binder}/{base}"),
        None => base,
    })
}

/// Every entry a binder can see: its own, then the shared codex. With no binder,
/// only the shared codex. Entries are sorted by kind, then name.
pub fn list_entities(content_dir: &Path, binder: Option<&str>) -> Result<Vec<Entity>, String> {
    if let Some(binder) = binder.filter(|b| !b.is_empty()) {
        ensure_binder(binder)?;
    }
    let mut roots = Vec::new();
    if let Some(binder) = binder.filter(|b| !b.is_empty() && *b != CODEX_DIR) {
        roots.push(format!("{binder}/{CODEX_DIR}"));
    }
    roots.push(CODEX_DIR.to_string());

    let mut entities = Vec::new();
    for root in roots {
        let dir = content_dir.join(&root);
        if !dir.is_dir() {
            continue;
        }
        for file in fs::discover_files(&dir).map_err(|e| e.to_string())? {
            let name = format!("{root}/{}", file.name);
            match file.read_content() {
                Ok(content) => entities.push(parse_entity(&name, &content)),
                Err(e) => eprintln!("Could not read codex entry {name}: {e}"),
            }
        }
    }
    // A binder's own entry wins over a shared one of the same kind and name.
    let mut seen = HashSet::new();
    entities.retain(|e| seen.insert((e.kind, e.name.to_lowercase())));
    entities.sort_by(|a, b| {
        (a.kind as u8, a.name.to_lowercase()).cmp(&(b.kind as u8, b.name.to_lowercase()))
    });
    Ok(entities)
}

/// Writes an entry, moving its file when the name, kind or scope changed.
/// `original_path` is where it was read from; `None` creates a new entry.
pub fn save_entity(
    content_dir: &Path,
    input: &EntityInput,
    original_path: Option<&str>,
) -> Result<Entity, String> {
    let target = entry_path(input.scope.as_deref(), input.kind, &input.name)?;
    let target_path = content_dir.join(&target);

    let original = original_path.filter(|p| !p.is_empty());
    if let Some(original) = original {
        ensure_relative(original)?;
        if !is_codex_name(original) {
            return Err(format!("\"{original}\" is not a codex entry"));
        }
    }
    let existing = match original {
        Some(original) => std::fs::read_to_string(content_dir.join(original)).ok(),
        None => None,
    };

    let moving = original.is_some_and(|o| o != target);
    let case_only = original.is_some_and(|o| o != target && o.to_lowercase() == target.to_lowercase());
    if (original.is_none() || moving) && target_path.exists() && !case_only {
        return Err(format!("\"{}\" is already in the codex", input.name.trim()));
    }

    let content = render_entity(input, existing.as_deref());
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if let (true, Some(original)) = (moving, original) {
        std::fs::rename(content_dir.join(original), &target_path).map_err(|e| e.to_string())?;
        prune_empty_dirs(content_dir, original);
    }
    std::fs::write(&target_path, &content).map_err(|e| e.to_string())?;
    Ok(parse_entity(&target, &content))
}

pub fn delete_entity(content_dir: &Path, path: &str) -> Result<(), String> {
    ensure_relative(path)?;
    if !is_codex_name(path) {
        return Err(format!("\"{path}\" is not a codex entry"));
    }
    std::fs::remove_file(content_dir.join(path)).map_err(|e| e.to_string())?;
    prune_empty_dirs(content_dir, path);
    Ok(())
}

/// Removes folders left empty under a codex entry's old path, up to and
/// including the `Codex` folder, so an emptied codex doesn't keep a binder
/// from being deleted. Stops at the first folder that still holds anything.
fn prune_empty_dirs(content_dir: &Path, path: &str) {
    let segments: Vec<&str> = path.split('/').collect();
    let Some(codex_at) = segments.iter().position(|s| *s == CODEX_DIR) else { return };
    for end in (codex_at + 1..segments.len()).rev() {
        let dir = content_dir.join(segments[..end].join("/"));
        let empty = std::fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(false);
        if !empty || std::fs::remove_dir(&dir).is_err() {
            break;
        }
    }
}

// -------------------------------------------------------
//  FINDING MENTIONS
// -------------------------------------------------------

/// Lowercases without changing any character's byte length, so offsets found in
/// folded text point at the same characters in the original.
fn fold_case(text: &str) -> String {
    text.chars()
        .map(|c| {
            let mut lower = c.to_lowercase();
            match (lower.next(), lower.next()) {
                (Some(l), None) if l.len_utf8() == c.len_utf8() => l,
                _ => c,
            }
        })
        .collect()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// One name or alias found in a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// Index into the entities the matcher was built from.
    pub entity: usize,
    pub start: usize,
    pub end: usize,
}

/// Finds entries' names and aliases in text, whole words only. Where names
/// overlap, the longest wins ("Iron Citadel" over "Citadel").
pub struct Matcher {
    sensitive: Option<(AhoCorasick, Vec<usize>)>,
    insensitive: Option<(AhoCorasick, Vec<usize>)>,
}

impl Matcher {
    pub fn new(entities: &[Entity]) -> Self {
        let mut sensitive = (Vec::new(), Vec::new());
        let mut insensitive = (Vec::new(), Vec::new());
        for (i, entity) in entities.iter().enumerate() {
            for term in entity.terms() {
                match entity.match_mode {
                    MatchMode::CaseSensitive => {
                        sensitive.0.push(term.to_string());
                        sensitive.1.push(i);
                    }
                    MatchMode::Insensitive => {
                        insensitive.0.push(fold_case(term));
                        insensitive.1.push(i);
                    }
                    MatchMode::Off => {}
                }
            }
        }
        let build = |(patterns, owners): (Vec<String>, Vec<usize>)| {
            if patterns.is_empty() {
                return None;
            }
            AhoCorasick::builder()
                .match_kind(MatchKind::Standard)
                .build(&patterns)
                .ok()
                .map(|ac| (ac, owners))
        };
        Matcher { sensitive: build(sensitive), insensitive: build(insensitive) }
    }

    /// Every mention in `text`, in order, none overlapping.
    pub fn find(&self, text: &str) -> Vec<Hit> {
        let mut hits = Vec::new();
        let folded = if self.insensitive.is_some() { fold_case(text) } else { String::new() };
        let searches = [(self.sensitive.as_ref(), text), (self.insensitive.as_ref(), folded.as_str())];
        for (automaton, haystack) in searches {
            let Some((ac, owners)) = automaton else { continue };
            for m in ac.find_overlapping_iter(haystack) {
                let (start, end) = (m.start(), m.end());
                let before = text[..start].chars().next_back();
                let after = text[end..].chars().next();
                if before.is_some_and(is_word_char) || after.is_some_and(is_word_char) {
                    continue;
                }
                hits.push(Hit { entity: owners[m.pattern().as_usize()], start, end });
            }
        }
        hits.sort_by(|a, b| a.start.cmp(&b.start).then((b.end - b.start).cmp(&(a.end - a.start))));
        let mut kept: Vec<Hit> = Vec::with_capacity(hits.len());
        for hit in hits {
            if kept.last().is_none_or(|last| hit.start >= last.end) {
                kept.push(hit);
            }
        }
        kept
    }

    /// How many times each entity is mentioned, by index.
    pub fn counts(&self, text: &str) -> HashMap<usize, usize> {
        let mut counts = HashMap::new();
        for hit in self.find(text) {
            *counts.entry(hit.entity).or_insert(0) += 1;
        }
        counts
    }
}

/// How often each entry is mentioned in `text`, most mentioned first.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mention {
    pub path: String,
    pub count: usize,
    /// Byte offset of the first mention.
    pub first: usize,
}

pub fn detect(entities: &[Entity], text: &str) -> Vec<Mention> {
    let matcher = Matcher::new(entities);
    let mut by_entity: HashMap<usize, Mention> = HashMap::new();
    for hit in matcher.find(text) {
        by_entity
            .entry(hit.entity)
            .or_insert_with(|| Mention { path: entities[hit.entity].path.clone(), count: 0, first: hit.start })
            .count += 1;
    }
    let mut mentions: Vec<Mention> = by_entity.into_values().collect();
    mentions.sort_by(|a, b| b.count.cmp(&a.count).then(a.first.cmp(&b.first)));
    mentions
}

// -------------------------------------------------------
//  MATRIX
// -------------------------------------------------------

/// Mentions of each entry in one writing, keyed by entry path.
#[derive(Debug, Clone, Serialize)]
pub struct SceneMentions {
    pub name: String,
    pub counts: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Matrix {
    pub entities: Vec<Entity>,
    /// Every writing in the binder, whether or not it mentions anything, sorted
    /// by name. The frontend puts them in binder order.
    pub scenes: Vec<SceneMentions>,
}

pub fn matrix(entities: Vec<Entity>, writings: Vec<(String, String)>) -> Matrix {
    let matcher = Matcher::new(&entities);
    let mut scenes: Vec<SceneMentions> = writings
        .into_iter()
        .map(|(name, content)| {
            let counts = matcher
                .counts(&content)
                .into_iter()
                .map(|(i, n)| (entities[i].path.clone(), n))
                .collect();
            SceneMentions { name, counts }
        })
        .collect();
    scenes.sort_by(|a, b| a.name.cmp(&b.name));
    Matrix { entities, scenes }
}

// -------------------------------------------------------
//  DRAFTING A SHEET
// -------------------------------------------------------

/// A passage from a writing that mentions an entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Passage {
    pub writing: String,
    pub text: String,
}

/// Paragraphs that mention `entity`, from `writings` in the order given, until
/// `budget` characters are used. Front matter is skipped.
pub fn gather_passages(entity: &Entity, writings: &[(String, String)], budget: usize) -> Vec<Passage> {
    // Match on the name and aliases even when detection is off for this entry.
    let mut probe = entity.clone();
    if probe.match_mode == MatchMode::Off {
        probe.match_mode = MatchMode::CaseSensitive;
    }
    let matcher = Matcher::new(std::slice::from_ref(&probe));

    let mut passages = Vec::new();
    let mut used = 0;
    for (name, content) in writings {
        let (_, body) = split_front_matter(content);
        for paragraph in body.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
            if matcher.find(paragraph).is_empty() {
                continue;
            }
            let text = truncate(paragraph, 1_200);
            if used + text.len() > budget {
                return passages;
            }
            used += text.len();
            passages.push(Passage { writing: name.clone(), text });
        }
    }
    passages
}

/// What the model suggests for an entry's empty fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SheetDraft {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub fields: HashMap<String, String>,
}

impl SheetDraft {
    /// Drops blank suggestions and any for fields that weren't asked about.
    pub fn keep_only(mut self, keys: &[String], want_summary: bool) -> Self {
        let wanted: HashSet<String> = keys.iter().map(|k| k.trim().to_lowercase()).collect();
        self.fields = self
            .fields
            .into_iter()
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .filter(|(k, v)| !v.is_empty() && wanted.contains(&k.to_lowercase()))
            .collect();
        self.summary = if want_summary { self.summary.trim().to_string() } else { String::new() };
        self
    }
}

/// The user turn asking the model to fill `keys` (and the summary, if empty)
/// from `passages`.
pub fn draft_message(entity: &Entity, keys: &[String], passages: &[Passage]) -> String {
    let mut sheet = render_for_model(entity, true);
    if entity.summary.is_empty() {
        sheet.push_str("\n(no summary yet)");
    }
    let mut asked: Vec<String> = keys.iter().map(|k| format!("\"{}\"", k.trim())).collect();
    if entity.summary.is_empty() {
        asked.insert(0, "the summary".to_string());
    }
    let excerpts = passages
        .iter()
        .map(|p| format!("<passage writing=\"{}\">\n{}\n</passage>", attr(&crate::memory::pretty_title(&p.writing)), p.text))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "<sheet>\n{sheet}\n</sheet>\n\nFill in: {}\n\n<passages>\n{excerpts}\n</passages>",
        asked.join(", ")
    )
}

// -------------------------------------------------------
//  CHAT CONTEXT
// -------------------------------------------------------

/// What a chat reply can draw entries from, besides the codex itself.
pub struct ContextSources<'a> {
    pub message: &'a str,
    /// Earlier turns, oldest first.
    pub history: &'a [String],
    pub document: Option<&'a str>,
    /// Entry names the user took out of the conversation.
    pub exclude: &'a [String],
    /// Entry names the user asked to keep in.
    pub pin: &'a [String],
    /// Whether entries marked pinned on their sheet always come along. Off for
    /// edits, which only need the entries the passage names.
    pub use_pinned: bool,
}

/// The entries picked for a reply and the block describing them to the model.
#[derive(Debug, Default)]
pub struct ContextBlock {
    pub text: String,
    /// Entries included, in order, and whether each got its full sheet.
    pub used: Vec<(Entity, bool)>,
}

/// Ranks entries by how relevant they are to this reply: named in the message
/// first, then pinned, then carried over from the conversation, then by how
/// often the open document mentions them, then linked to a top entry.
pub fn rank_for_context(entities: &[Entity], sources: &ContextSources) -> Vec<usize> {
    let matcher = Matcher::new(entities);
    let lower = |names: &[String]| names.iter().map(|n| n.trim().to_lowercase()).collect::<HashSet<_>>();
    let (exclude, pin) = (lower(sources.exclude), lower(sources.pin));

    let mut scores: HashMap<usize, i64> = HashMap::new();
    for (i, n) in matcher.counts(sources.message) {
        *scores.entry(i).or_default() += 1_000 + n as i64;
    }
    for (i, entity) in entities.iter().enumerate() {
        if (sources.use_pinned && entity.pinned) || pin.contains(&entity.name.to_lowercase()) {
            *scores.entry(i).or_default() += 500;
        }
    }
    // Later turns count more, so "she" most likely means whoever was just discussed.
    for (age, turn) in sources.history.iter().rev().enumerate() {
        for i in matcher.counts(turn).into_keys() {
            *scores.entry(i).or_default() += 200_i64.saturating_sub(age as i64 * 30).max(20);
        }
    }
    if let Some(document) = sources.document {
        for (i, n) in matcher.counts(document) {
            *scores.entry(i).or_default() += 10 + n.min(40) as i64;
        }
    }

    let by_name: HashMap<String, usize> = entities
        .iter()
        .enumerate()
        .flat_map(|(i, e)| e.terms().into_iter().map(move |t| (t.to_lowercase(), i)))
        .collect();
    let named_in_message: Vec<usize> = scores.iter().filter(|(_, s)| **s >= 1_000).map(|(i, _)| *i).collect();
    for i in named_in_message {
        for rel in &entities[i].relationships {
            if let Some(&j) = by_name.get(&rel.to.trim().to_lowercase()) {
                scores.entry(j).or_insert(5);
            }
        }
    }

    let mut ranked: Vec<(usize, i64)> = scores
        .into_iter()
        .filter(|(i, _)| !exclude.contains(&entities[*i].name.to_lowercase()))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(entities[a.0].name.cmp(&entities[b.0].name)));
    ranked.into_iter().map(|(i, _)| i).collect()
}

fn attr(value: &str) -> String {
    value.replace('"', "'").replace('\n', " ")
}

fn truncate(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((idx, _)) => format!("{}…", &text[..idx]),
        None => text.to_string(),
    }
}

fn render_for_model(entity: &Entity, full: bool) -> String {
    let mut open = format!("<entity name=\"{}\" kind=\"{}\"", attr(&entity.name), entity.kind.as_str());
    if !entity.aliases.is_empty() {
        open.push_str(&format!(" aliases=\"{}\"", attr(&entity.aliases.join(", "))));
    }
    open.push('>');

    let mut lines = vec![open];
    if !entity.summary.is_empty() {
        lines.push(format!("Summary: {}", entity.summary));
    }
    if full {
        for field in &entity.fields {
            lines.push(format!("{}: {}", capitalize(&field.key), field.value));
        }
        if !entity.relationships.is_empty() {
            let rels: Vec<String> = entity
                .relationships
                .iter()
                .map(|r| if r.kind.is_empty() { r.to.clone() } else { format!("{} ({})", r.to, r.kind) })
                .collect();
            lines.push(format!("Relationships: {}", rels.join("; ")));
        }
        if !entity.notes.is_empty() {
            lines.push(format!("Notes: {}", truncate(&entity.notes, CONTEXT_NOTES_CHARS)));
        }
    }
    lines.push("</entity>".to_string());
    lines.join("\n")
}

fn capitalize(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

const CONTEXT_PREAMBLE: &str = "Story codex: reference sheets for characters, places, groups and things in the user's story. Treat them as canonical facts and don't contradict them.";

/// Picks the entries relevant to a reply and describes them to the model within
/// `budget` characters. The top entries get their whole sheet; once that no
/// longer fits, the rest get just their summary.
pub fn build_context(entities: &[Entity], sources: &ContextSources, budget: usize) -> ContextBlock {
    let ranked = rank_for_context(entities, sources);
    if ranked.is_empty() {
        return ContextBlock::default();
    }

    let mut used = Vec::new();
    let mut parts = Vec::new();
    let mut size = CONTEXT_PREAMBLE.len() + "<codex>\n\n</codex>".len();
    for i in ranked {
        if used.len() >= CONTEXT_MAX_ENTITIES {
            break;
        }
        let entity = &entities[i];
        for full in [true, false] {
            let text = render_for_model(entity, full);
            if size + text.len() + 2 <= budget {
                size += text.len() + 2;
                parts.push(text);
                used.push((entity.clone(), full));
                break;
            }
        }
    }
    if parts.is_empty() {
        return ContextBlock::default();
    }
    ContextBlock {
        text: format!("<codex>\n{CONTEXT_PREAMBLE}\n\n{}\n</codex>", parts.join("\n\n")),
        used,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn entity(name: &str, aliases: &[&str], mode: MatchMode) -> Entity {
        Entity {
            path: format!("Novel/Codex/Characters/{name}.md"),
            scope: Some("Novel".into()),
            name: name.into(),
            kind: EntityKind::Character,
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            summary: format!("{name} summary."),
            fields: Vec::new(),
            relationships: Vec::new(),
            match_mode: mode,
            pinned: false,
            notes: String::new(),
        }
    }

    fn input(name: &str) -> EntityInput {
        EntityInput {
            scope: Some("Novel".into()),
            name: name.into(),
            kind: EntityKind::Character,
            aliases: vec!["Ari".into()],
            summary: "Exiled heir.".into(),
            fields: vec![Field { key: "appearance".into(), value: "Grey eyes".into() }],
            relationships: vec![Relationship { to: "Kael".into(), kind: "rival".into() }],
            match_mode: MatchMode::CaseSensitive,
            pinned: false,
            notes: "Hates the sea.".into(),
        }
    }

    #[test]
    fn codex_names_are_recognised_at_top_level_and_in_binders() {
        assert!(is_codex_name("Codex/Characters/Aria.md"));
        assert!(is_codex_name("Novel/Codex/Characters/Aria.md"));
        assert!(!is_codex_name("Codex.md"));
        assert!(!is_codex_name("Novel/Codex.md"));
        assert!(!is_codex_name("Novel/Ch 1/Codex/Scene.md"));
        assert!(is_codex_folder("Novel/Codex"));
        assert!(!is_codex_folder("Novel/Chapter 1"));
    }

    #[test]
    fn paths_outside_the_content_directory_are_refused() {
        let dir = tempdir().unwrap();
        assert!(delete_entity(dir.path(), "Novel/Codex/../../../etc/passwd").is_err());
        assert!(delete_entity(dir.path(), "/Codex/Characters/Aria.md").is_err());
        let mut sneaky = input("Aria");
        sneaky.scope = Some("../outside".into());
        assert!(save_entity(dir.path(), &sneaky, None).is_err());
        assert!(list_entities(dir.path(), Some("..")).is_err());
        assert!(ensure_not_reserved("Novel/Codex", false).is_err());
        assert!(ensure_not_reserved("Codex", false).is_err());
        assert!(ensure_not_reserved("Novel/Codex/Scene.md", true).is_err());
        assert!(ensure_not_reserved("Novel/Chapter 1/Codex notes.md", true).is_ok());
    }

    #[test]
    fn same_name_different_kind_both_listed() {
        let dir = tempdir().unwrap();
        save_entity(dir.path(), &input("Raven"), None).unwrap();
        let mut item = input("Raven");
        item.kind = EntityKind::Item;
        save_entity(dir.path(), &item, None).unwrap();
        assert_eq!(list_entities(dir.path(), Some("Novel")).unwrap().len(), 2);
    }

    #[test]
    fn entries_round_trip_and_keep_unknown_keys() {
        let existing = "---\ncodex: character\ntags: [hero]\n---\nold notes\n";
        let content = render_entity(&input("Aria"), Some(existing));
        assert!(content.contains("tags:"));
        let parsed = parse_entity("Novel/Codex/Characters/Aria.md", &content);
        assert_eq!(parsed.name, "Aria");
        assert_eq!(parsed.scope.as_deref(), Some("Novel"));
        assert_eq!(parsed.aliases, vec!["Ari".to_string()]);
        assert_eq!(parsed.fields, input("Aria").fields);
        assert_eq!(parsed.relationships, input("Aria").relationships);
        assert_eq!(parsed.notes, "Hates the sea.");
    }

    #[test]
    fn kind_falls_back_to_folder_and_notes_without_front_matter() {
        let parsed = parse_entity("Codex/Locations/Veyra.md", "Just notes.");
        assert_eq!(parsed.kind, EntityKind::Location);
        assert_eq!(parsed.scope, None);
        assert_eq!(parsed.notes, "Just notes.");
    }

    #[test]
    fn save_moves_renamed_entries_and_refuses_clashes() {
        let dir = tempdir().unwrap();
        let saved = save_entity(dir.path(), &input("Aria"), None).unwrap();
        assert_eq!(saved.path, "Novel/Codex/Characters/Aria.md");
        assert!(save_entity(dir.path(), &input("Aria"), None).is_err());

        let renamed = save_entity(dir.path(), &input("Arya"), Some(&saved.path)).unwrap();
        assert!(!dir.path().join(&saved.path).exists());
        assert!(dir.path().join(&renamed.path).exists());

        let listed = list_entities(dir.path(), Some("Novel")).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Arya");
    }

    #[test]
    fn deleting_the_last_entry_removes_the_empty_codex_folders() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Novel")).unwrap();
        std::fs::write(dir.path().join("Novel/Scene.md"), "x").unwrap();
        let aria = save_entity(dir.path(), &input("Aria"), None).unwrap();
        let mut place = input("Veyra");
        place.kind = EntityKind::Location;
        let veyra = save_entity(dir.path(), &place, None).unwrap();

        delete_entity(dir.path(), &aria.path).unwrap();
        assert!(!dir.path().join("Novel/Codex/Characters").exists());
        assert!(dir.path().join("Novel/Codex/Locations").exists());

        delete_entity(dir.path(), &veyra.path).unwrap();
        assert!(!dir.path().join("Novel/Codex").exists());
        assert!(dir.path().join("Novel/Scene.md").exists());
    }

    #[test]
    fn binder_entries_shadow_shared_ones() {
        let dir = tempdir().unwrap();
        save_entity(dir.path(), &input("Aria"), None).unwrap();
        let mut shared = input("Aria");
        shared.scope = None;
        save_entity(dir.path(), &shared, None).unwrap();
        let listed = list_entities(dir.path(), Some("Novel")).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].scope.as_deref(), Some("Novel"));
        assert_eq!(list_entities(dir.path(), None).unwrap()[0].scope, None);
    }

    #[test]
    fn matcher_finds_whole_words_and_possessives() {
        let entities = vec![entity("Aria", &["Ari"], MatchMode::CaseSensitive)];
        let counts = Matcher::new(&entities).counts("Aria's sword. Ari ran. Arianne and aria stayed.");
        assert_eq!(counts.get(&0), Some(&2));
    }

    #[test]
    fn matcher_prefers_longest_names_and_honours_case_modes() {
        let entities = vec![
            entity("The Iron Citadel", &[], MatchMode::Insensitive),
            entity("Citadel", &[], MatchMode::CaseSensitive),
            entity("Will", &[], MatchMode::Off),
        ];
        let counts = Matcher::new(&entities).counts("They reached the iron citadel. Will the Citadel hold?");
        assert_eq!(counts.get(&0), Some(&1));
        assert_eq!(counts.get(&1), Some(&1));
        assert_eq!(counts.get(&2), None);
    }

    #[test]
    fn a_longer_name_that_fails_the_word_check_does_not_hide_a_shorter_one() {
        let entities = vec![
            entity("Kael", &[], MatchMode::CaseSensitive),
            entity("Kael Dor", &[], MatchMode::CaseSensitive),
        ];
        let hits = Matcher::new(&entities).find("Kael Dora waved.");
        assert_eq!(hits, vec![Hit { entity: 0, start: 0, end: 4 }]);
    }

    #[test]
    fn insensitive_matching_keeps_offsets_on_non_ascii_text() {
        let entities = vec![entity("Éloise", &[], MatchMode::Insensitive)];
        let text = "İ saw éloise.";
        let hits = Matcher::new(&entities).find(text);
        assert_eq!(hits.len(), 1);
        assert_eq!(&text[hits[0].start..hits[0].end], "éloise");
    }

    #[test]
    fn context_ranks_message_then_pins_then_history_then_document() {
        let mut entities = vec![
            entity("Aria", &[], MatchMode::CaseSensitive),
            entity("Kael", &[], MatchMode::CaseSensitive),
            entity("Mara", &[], MatchMode::CaseSensitive),
            entity("Oren", &[], MatchMode::CaseSensitive),
            entity("Ghost", &[], MatchMode::CaseSensitive),
        ];
        entities[3].pinned = true;
        let history = vec!["Tell me about Mara".to_string()];
        let sources = ContextSources {
            message: "What does Kael want?",
            history: &history,
            document: Some("Aria walked. Aria ran."),
            exclude: &[],
            pin: &[],
            use_pinned: true,
        };
        let ranked: Vec<&str> = rank_for_context(&entities, &sources)
            .into_iter()
            .map(|i| entities[i].name.as_str())
            .collect();
        assert_eq!(ranked, vec!["Kael", "Oren", "Mara", "Aria"]);
    }

    #[test]
    fn pinned_sheets_can_be_left_out() {
        let mut entities = vec![entity("Aria", &[], MatchMode::CaseSensitive), entity("Oren", &[], MatchMode::CaseSensitive)];
        entities[1].pinned = true;
        let pin = vec!["aria".to_string()];
        let sources = ContextSources { message: "No names here.", history: &[], document: None, exclude: &[], pin: &pin, use_pinned: false };
        // A pin from the chat still counts; the sheet's own pinned flag doesn't.
        let ranked: Vec<&str> = rank_for_context(&entities, &sources).into_iter().map(|i| entities[i].name.as_str()).collect();
        assert_eq!(ranked, vec!["Aria"]);
    }

    #[test]
    fn context_skips_excluded_and_follows_relationships() {
        let mut entities = vec![
            entity("Aria", &[], MatchMode::CaseSensitive),
            entity("Kael", &[], MatchMode::CaseSensitive),
            entity("Veyra", &[], MatchMode::Off),
        ];
        entities[0].relationships = vec![Relationship { to: "veyra".into(), kind: "home".into() }];
        let exclude = vec!["kael".to_string()];
        let sources = ContextSources {
            message: "Aria and Kael argue.",
            history: &[],
            document: None,
            exclude: &exclude,
            pin: &[],
            use_pinned: true,
        };
        let ranked: Vec<&str> = rank_for_context(&entities, &sources)
            .into_iter()
            .map(|i| entities[i].name.as_str())
            .collect();
        assert_eq!(ranked, vec!["Aria", "Veyra"]);
    }

    #[test]
    fn context_falls_back_to_summaries_within_budget() {
        let mut big = entity("Aria", &[], MatchMode::CaseSensitive);
        big.notes = "x".repeat(2_000);
        big.fields = vec![Field { key: "arc".into(), value: "y".repeat(400) }];
        let entities = vec![big, entity("Kael", &[], MatchMode::CaseSensitive)];
        let sources = ContextSources { message: "Aria, Kael", history: &[], document: None, exclude: &[], pin: &[], use_pinned: true };

        let block = build_context(&entities, &sources, 600);
        let used: Vec<(&str, bool)> = block.used.iter().map(|(e, full)| (e.name.as_str(), *full)).collect();
        assert_eq!(used, vec![("Aria", false), ("Kael", true)]);
        assert!(block.text.len() <= 600);
        assert!(block.text.starts_with("<codex>"));

        let none = ContextSources { message: "hello", history: &[], document: None, exclude: &[], pin: &[], use_pinned: true };
        assert!(build_context(&entities, &none, 600).text.is_empty());
    }

    #[test]
    fn passages_are_the_paragraphs_that_mention_the_entry() {
        let mut aria = entity("Aria", &["Ari"], MatchMode::Off);
        aria.summary.clear();
        let writings = vec![
            ("Novel/a.md".to_string(), "---\nsynopsis: Aria waits\n---\nRain fell.\n\nAri drew her sword.".to_string()),
            ("Novel/b.md".to_string(), "Aria laughed.\n\nKael did not.".to_string()),
        ];
        let passages = gather_passages(&aria, &writings, 1_000);
        let texts: Vec<&str> = passages.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, vec!["Ari drew her sword.", "Aria laughed."]);
        assert_eq!(gather_passages(&aria, &writings, 25).len(), 1);

        let message = draft_message(&aria, &["appearance".to_string()], &passages);
        assert!(message.contains("Fill in: the summary, \"appearance\""));
        assert!(message.contains("<passage writing=\"Novel › b\">"));
    }

    #[test]
    fn drafts_keep_only_what_was_asked_for() {
        let draft = SheetDraft {
            summary: " A swordswoman. ".into(),
            fields: HashMap::from([
                ("Appearance".to_string(), "Grey eyes".to_string()),
                ("arc".to_string(), " ".to_string()),
                ("favourite food".to_string(), "Figs".to_string()),
            ]),
        }
        .keep_only(&["appearance".to_string(), "arc".to_string()], true);
        assert_eq!(draft.summary, "A swordswoman.");
        assert_eq!(draft.fields, HashMap::from([("Appearance".to_string(), "Grey eyes".to_string())]));
    }

    #[test]
    fn matrix_counts_mentions_per_scene() {
        let entities = vec![entity("Aria", &[], MatchMode::CaseSensitive), entity("Kael", &[], MatchMode::CaseSensitive)];
        let m = matrix(
            entities,
            vec![
                ("Novel/b.md".into(), "Kael.".into()),
                ("Novel/a.md".into(), "Aria and Aria and Kael.".into()),
            ],
        );
        assert_eq!(m.scenes[0].name, "Novel/a.md");
        assert_eq!(m.scenes[0].counts.get("Novel/Codex/Characters/Aria.md"), Some(&2));
        assert_eq!(m.scenes[1].counts.get("Novel/Codex/Characters/Aria.md"), None);
    }
}
