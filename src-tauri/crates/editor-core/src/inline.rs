//! Inline content: the text inside a paragraph, heading, table cell or list
//! paragraph.
//!
//! Markdown nests inline markup (`**bold _and italic_**`), but nesting is a
//! spelling of the source, not a property of the text: what the writer sees is
//! a run of characters carrying a set of marks. Inline content here is
//! therefore a **flat list of runs**, not a tree.
//!
//! That choice is what makes editing cheap and correct:
//!
//! * inserting or deleting at an offset touches at most two runs;
//! * toggling bold over a selection is a mark-set change on the runs it
//!   covers, with no tree to rebuild;
//! * "is the selection bold?" is a scan, not a walk up ancestors;
//! * two spellings of the same text (`**a**b**c**` and `**a**` + `b` +
//!   `**c**`) normalise to the same content, so round-tripping is stable.
//!
//! The Markdown serializer (Phase 3) rebuilds nesting on the way out.
//!
//! # Offsets
//!
//! Offsets count **Unicode scalar values** (Rust `char`s), not bytes and not
//! UTF-16 code units. Atoms — an inline image, a hard break — count as one.
//! Bytes would put positions inside multi-byte characters; UTF-16 would bake
//! a JavaScript detail into the engine. The Tauri layer converts at the
//! boundary, where the frontend's units are known.

use serde::{Deserialize, Serialize};

/// The marks a run of text can carry. Code is a mark rather than a node so
/// `` `code` `` inside a link, or a partially code-spanned selection, needs no
/// special case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MarkSet(pub u8);

impl MarkSet {
    pub const NONE: MarkSet = MarkSet(0);
    pub const BOLD: MarkSet = MarkSet(1 << 0);
    pub const ITALIC: MarkSet = MarkSet(1 << 1);
    pub const CODE: MarkSet = MarkSet(1 << 2);
    pub const STRIKE: MarkSet = MarkSet(1 << 3);

    pub fn contains(self, other: MarkSet) -> bool {
        other.0 != 0 && self.0 & other.0 == other.0
    }

    pub fn with(self, other: MarkSet) -> MarkSet {
        MarkSet(self.0 | other.0)
    }

    pub fn without(self, other: MarkSet) -> MarkSet {
        MarkSet(self.0 & !other.0)
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// A link target. Kept on the run rather than wrapping it, so a link and the
/// marks inside it are independent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl Link {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            title: None,
        }
    }
}

/// An image, inline or as a block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageNode {
    pub source: String,
    #[serde(default)]
    pub alt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl ImageNode {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            alt: String::new(),
            title: None,
        }
    }
}

/// One piece of inline content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    /// A run of characters sharing marks and link target.
    Text {
        text: String,
        #[serde(default)]
        marks: MarkSet,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<Link>,
    },
    Image(ImageNode),
    /// Raw inline HTML (`<br>`, `<span …>`), kept verbatim. Treated as one
    /// indivisible unit so an edit can never cut a tag in half.
    Html(String),
    /// A line break inside a block (`\` or two trailing spaces in Markdown).
    HardBreak,
}

impl Inline {
    pub fn text(text: impl Into<String>) -> Self {
        Inline::Text {
            text: text.into(),
            marks: MarkSet::NONE,
            link: None,
        }
    }

    pub fn marked(text: impl Into<String>, marks: MarkSet) -> Self {
        Inline::Text {
            text: text.into(),
            marks,
            link: None,
        }
    }

    pub fn link(text: impl Into<String>, url: impl Into<String>) -> Self {
        Inline::Text {
            text: text.into(),
            marks: MarkSet::NONE,
            link: Some(Link::new(url)),
        }
    }

    /// Length in offset units: the character count of a text run, 1 for an atom.
    pub fn len(&self) -> usize {
        match self {
            Inline::Text { text, .. } => text.chars().count(),
            Inline::Image(_) | Inline::Html(_) | Inline::HardBreak => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The marks on this piece; atoms carry none.
    pub fn marks(&self) -> MarkSet {
        match self {
            Inline::Text { marks, .. } => *marks,
            _ => MarkSet::NONE,
        }
    }

    /// Plain text, as search, word counts and embeddings see it. Atoms
    /// contribute their alt text (images) or a newline (hard breaks).
    fn write_plain_text(&self, out: &mut String) {
        match self {
            Inline::Text { text, .. } => out.push_str(text),
            Inline::Image(image) => out.push_str(&image.alt),
            // Raw HTML is markup, not prose: it stays out of plain text.
            Inline::Html(_) => {}
            Inline::HardBreak => out.push('\n'),
        }
    }

    /// The piece cut down to `range` (in this piece's own offsets). An atom is
    /// kept whole or dropped; it is never split.
    fn slice(&self, start: usize, end: usize) -> Option<Inline> {
        match self {
            Inline::Text { text, marks, link } => {
                let taken: String = text.chars().skip(start).take(end - start).collect();
                (!taken.is_empty()).then(|| Inline::Text {
                    text: taken,
                    marks: *marks,
                    link: link.clone(),
                })
            }
            atom => (start == 0 && end == 1).then(|| atom.clone()),
        }
    }
}

/// The inline content of one block: a flat, normalised sequence of runs.
///
/// Normalised means no empty text runs and no two adjacent text runs that
/// share marks and link, so equal text always compares equal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InlineContent(Vec<Inline>);

impl InlineContent {
    pub fn new(pieces: Vec<Inline>) -> Self {
        let mut content = Self(pieces);
        content.normalize();
        content
    }

    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Plain unmarked text, the common case.
    pub fn from_text(text: impl Into<String>) -> Self {
        Self::new(vec![Inline::text(text)])
    }

    pub fn pieces(&self) -> &[Inline] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(Inline::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for piece in &self.0 {
            piece.write_plain_text(&mut out);
        }
        out
    }

    /// Drops empty runs and merges neighbours that carry the same formatting.
    fn normalize(&mut self) {
        let mut out: Vec<Inline> = Vec::with_capacity(self.0.len());
        for piece in self.0.drain(..) {
            if piece.is_empty() {
                continue;
            }
            match (out.last_mut(), &piece) {
                (
                    Some(Inline::Text {
                        text: prev,
                        marks: pm,
                        link: pl,
                    }),
                    Inline::Text { text, marks, link },
                ) if pm == marks && pl == link => prev.push_str(text),
                _ => out.push(piece),
            }
        }
        self.0 = out;
    }

    /// The content between two offsets. Offsets past the end clamp.
    pub fn slice(&self, start: usize, end: usize) -> InlineContent {
        let len = self.len();
        let (start, end) = (start.min(len), end.min(len));
        if start >= end {
            return Self::empty();
        }

        let mut out = Vec::new();
        let mut at = 0;
        for piece in &self.0 {
            let piece_len = piece.len();
            let piece_end = at + piece_len;
            if piece_end > start && at < end {
                let from = start.saturating_sub(at);
                let to = (end - at).min(piece_len);
                if let Some(part) = piece.slice(from, to) {
                    out.push(part);
                }
            }
            at = piece_end;
            if at >= end {
                break;
            }
        }
        Self::new(out)
    }

    /// Replaces `start..end` with `content` and returns what was there, so the
    /// change can be inverted exactly.
    pub fn replace_range(
        &mut self,
        start: usize,
        end: usize,
        content: InlineContent,
    ) -> InlineContent {
        let len = self.len();
        let (start, end) = (start.min(len), end.min(len));
        let (start, end) = (start.min(end), end.max(start));

        let removed = self.slice(start, end);
        let mut pieces = self.slice(0, start).0;
        pieces.extend(content.0);
        pieces.extend(self.slice(end, len).0);
        *self = Self::new(pieces);
        removed
    }

    /// Whether every text run in `start..end` carries `mark`. An empty range,
    /// or one holding only atoms, is not marked.
    pub fn has_mark_throughout(&self, start: usize, end: usize, mark: MarkSet) -> bool {
        let slice = self.slice(start, end);
        let runs: Vec<&Inline> = slice
            .0
            .iter()
            .filter(|p| matches!(p, Inline::Text { .. }))
            .collect();
        !runs.is_empty() && runs.iter().all(|p| p.marks().contains(mark))
    }

    /// Adds and removes marks over `start..end`, splitting runs at the edges.
    pub fn apply_marks(&mut self, start: usize, end: usize, add: MarkSet, remove: MarkSet) {
        let mut middle = self.slice(start, end);
        for piece in middle.0.iter_mut() {
            if let Inline::Text { marks, .. } = piece {
                *marks = marks.with(add).without(remove);
            }
        }
        self.replace_range(start, end, middle);
    }

    /// Sets or clears the link target over `start..end`.
    pub fn apply_link(&mut self, start: usize, end: usize, link: Option<Link>) {
        let mut middle = self.slice(start, end);
        for piece in middle.0.iter_mut() {
            if let Inline::Text { link: slot, .. } = piece {
                *slot = link.clone();
            }
        }
        self.replace_range(start, end, middle);
    }

    /// The link at `offset`, looking at the run the offset sits inside.
    pub fn link_at(&self, offset: usize) -> Option<&Link> {
        let mut at = 0;
        for piece in &self.0 {
            let end = at + piece.len();
            if offset < end || (offset == end && offset == self.len()) {
                if let Inline::Text { link, .. } = piece {
                    return link.as_ref();
                }
                return None;
            }
            at = end;
        }
        None
    }

    /// The link covering `offset` on both sides, i.e. the one that typing
    /// there should join. A caret at either edge of a link inherits nothing,
    /// so typing just after a link doesn't silently extend it.
    pub fn link_spanning(&self, offset: usize) -> Option<&Link> {
        if offset == 0 || offset >= self.len() {
            return None;
        }
        let mut at = 0;
        for piece in &self.0 {
            let end = at + piece.len();
            if offset > at && offset < end {
                return match piece {
                    Inline::Text { link, .. } => link.as_ref(),
                    _ => None,
                };
            }
            at = end;
        }
        None
    }

    /// The marks that typing at `offset` should inherit: those of the run
    /// ending there, which is how editors behave at a mark's trailing edge.
    pub fn marks_at(&self, offset: usize) -> MarkSet {
        if offset == 0 {
            return self.0.first().map(Inline::marks).unwrap_or(MarkSet::NONE);
        }
        let mut at = 0;
        for piece in &self.0 {
            let end = at + piece.len();
            if offset <= end {
                return piece.marks();
            }
            at = end;
        }
        MarkSet::NONE
    }
}

impl From<&str> for InlineContent {
    fn from(text: &str) -> Self {
        Self::from_text(text)
    }
}
