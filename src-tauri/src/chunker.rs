//! Splits a note into passages for embedding.
//!
//! A passage never spans two sections: every heading starts a new one, and a
//! section longer than `CHUNK_SIZE_LIMIT` is split between blocks (or, for a
//! single huge paragraph, between sentences). Each passage remembers the
//! headings it sits under and the lines it covers, and a passage that
//! continues a section carries the previous passage's last sentence so its
//! embedding keeps the thread. YAML front matter is not indexed.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// Soft cap on a passage's text, in bytes. A passage only goes over when a
/// single sentence does.
pub const CHUNK_SIZE_LIMIT: usize = 1000;

/// Most text carried over from the previous passage of the same section.
const OVERLAP_LIMIT: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// The passage as written in the note.
    pub text: String,
    /// Headings above the passage, outermost first, e.g. `["Part 1", "Scene 2"]`.
    pub headings: Vec<String>,
    /// First and last line of the note the passage covers, 1-based.
    pub start_line: usize,
    pub end_line: usize,
    /// End of the previous passage in the same section; embedded with this
    /// one for context, never shown as part of it.
    pub overlap: String,
}

struct Block {
    range: Range<usize>,
    /// Set for headings: level and plain text.
    heading: Option<(HeadingLevel, String)>,
}

/// Top-level blocks of `text` with their byte ranges, front matter dropped.
fn blocks(text: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;

    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut heading: Option<(HeadingLevel, String)> = None;
    let mut in_metadata = false;

    for (event, range) in Parser::new_ext(text, options).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    start = range.start;
                    match tag {
                        Tag::Heading { level, .. } => heading = Some((level, String::new())),
                        Tag::MetadataBlock(_) => in_metadata = true,
                        _ => {}
                    }
                }
                depth += 1;
            }
            Event::End(tag) => {
                depth -= 1;
                if depth == 0 {
                    if in_metadata || matches!(tag, TagEnd::MetadataBlock(_)) {
                        in_metadata = false;
                    } else {
                        out.push(Block { range: start..range.end, heading: heading.take() });
                    }
                }
            }
            Event::Text(s) | Event::Code(s) => {
                if let Some((_, h)) = heading.as_mut() {
                    h.push_str(&s);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, h)) = heading.as_mut() {
                    h.push(' ');
                }
            }
            // Leaf blocks outside any container, e.g. a thematic break.
            _ if depth == 0 => out.push(Block { range, heading: None }),
            _ => {}
        }
    }
    out
}

/// Byte offset -> 1-based line number.
struct Lines(Vec<usize>);

impl Lines {
    fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Self(starts)
    }

    fn at(&self, offset: usize) -> usize {
        self.0.partition_point(|&start| start <= offset)
    }
}

/// Ranges of `text[range]` no longer than `limit` where possible, cut after
/// sentence ends or line breaks. A single sentence over the limit is cut at
/// the nearest char boundary.
fn split_long(text: &str, range: Range<usize>, limit: usize) -> Vec<Range<usize>> {
    let slice = &text[range.clone()];
    let mut cuts: Vec<usize> = slice
        .char_indices()
        .zip(slice.chars().skip(1))
        .filter(|((_, c), next)| matches!(c, '.' | '!' | '?' | '\n') && next.is_whitespace())
        .map(|((i, c), _)| i + c.len_utf8())
        .collect();
    cuts.push(slice.len());

    let mut pieces = Vec::new();
    let mut piece_start = 0;
    let mut last_cut = 0;
    for cut in cuts {
        if cut - piece_start > limit && last_cut > piece_start {
            pieces.push(piece_start..last_cut);
            piece_start = last_cut;
        }
        while cut - piece_start > limit {
            let mut end = piece_start + limit;
            while !slice.is_char_boundary(end) {
                end -= 1;
            }
            pieces.push(piece_start..end);
            piece_start = end;
        }
        last_cut = cut;
    }
    if piece_start < slice.len() {
        pieces.push(piece_start..slice.len());
    }

    pieces
        .into_iter()
        .map(|p| range.start + p.start..range.start + p.end)
        .filter(|p| !text[p.clone()].trim().is_empty())
        .collect()
}

/// The last sentence (or line) of `text`, capped at `OVERLAP_LIMIT` bytes.
fn tail(text: &str) -> String {
    let trimmed = text.trim_end();
    let from = trimmed
        .char_indices()
        .rev()
        .skip(1)
        .find(|&(_, c)| matches!(c, '.' | '!' | '?' | '\n'))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    let mut from = from.max(trimmed.len().saturating_sub(OVERLAP_LIMIT));
    while !trimmed.is_char_boundary(from) {
        from += 1;
    }
    trimmed[from..].trim().to_string()
}

struct Builder<'a> {
    text: &'a str,
    lines: Lines,
    chunks: Vec<Chunk>,
    headings: Vec<(HeadingLevel, String)>,
    /// Byte ranges of the pieces in the passage being built.
    pieces: Vec<Range<usize>>,
    size: usize,
    overlap: String,
}

impl Builder<'_> {
    fn path(&self) -> Vec<String> {
        self.headings.iter().map(|(_, h)| h.clone()).collect()
    }

    /// Ends the current passage. `continues` means the next passage is in the
    /// same section, so it carries this one's last sentence.
    fn flush(&mut self, continues: bool) {
        let (Some(first), Some(last)) = (self.pieces.first(), self.pieces.last()) else {
            return;
        };
        let (start, end) = (first.start, last.end);
        let text = self
            .pieces
            .iter()
            .map(|p| self.text[p.clone()].trim())
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunk = Chunk {
            headings: self.path(),
            start_line: self.lines.at(start),
            end_line: self.lines.at(end.saturating_sub(1).max(start)),
            overlap: std::mem::take(&mut self.overlap),
            text,
        };
        if continues {
            self.overlap = tail(&chunk.text);
        }
        self.chunks.push(chunk);
        self.pieces.clear();
        self.size = 0;
    }

    fn add(&mut self, range: Range<usize>) {
        let len = self.text[range.clone()].trim().len();
        if len == 0 {
            return;
        }
        if !self.pieces.is_empty() && self.size + len + 2 > CHUNK_SIZE_LIMIT {
            self.flush(true);
        }
        self.size += len + if self.pieces.is_empty() { 0 } else { 2 };
        self.pieces.push(range);
    }
}

pub fn chunk_markdown(text: &str) -> Vec<Chunk> {
    let mut b = Builder {
        text,
        lines: Lines::new(text),
        chunks: Vec::new(),
        headings: Vec::new(),
        pieces: Vec::new(),
        size: 0,
        overlap: String::new(),
    };

    for block in blocks(text) {
        if let Some((level, title)) = block.heading {
            b.flush(false);
            b.overlap.clear();
            b.headings.retain(|(l, _)| *l < level);
            let title = title.trim().to_string();
            if !title.is_empty() {
                b.headings.push((level, title));
            }
            b.add(block.range);
            continue;
        }
        for piece in split_long(text, block.range, CHUNK_SIZE_LIMIT) {
            b.add(piece);
        }
    }
    b.flush(false);
    b.chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(chunks: &[Chunk]) -> Vec<&str> {
        chunks.iter().map(|c| c.text.as_str()).collect()
    }

    #[test]
    fn headings_start_passages_and_build_a_path() {
        let note = "Intro line.\n\n# Part 1\n\nOpening.\n\n## Scene 2\n\nShe runs.\n\n# Part 2\n\nLater.\n";
        let chunks = chunk_markdown(note);
        assert_eq!(
            texts(&chunks),
            vec!["Intro line.", "# Part 1\n\nOpening.", "## Scene 2\n\nShe runs.", "# Part 2\n\nLater."]
        );
        assert!(chunks[0].headings.is_empty());
        assert_eq!(chunks[1].headings, vec!["Part 1"]);
        assert_eq!(chunks[2].headings, vec!["Part 1", "Scene 2"]);
        assert_eq!(chunks[3].headings, vec!["Part 2"]);
        assert!(chunks.iter().all(|c| c.overlap.is_empty()));
    }

    #[test]
    fn line_ranges_point_into_the_note() {
        let note = "# Title\n\nFirst para\nwraps here.\n\n## Next\n\nMore.";
        let chunks = chunk_markdown(note);
        assert_eq!((chunks[0].start_line, chunks[0].end_line), (1, 4));
        assert_eq!((chunks[1].start_line, chunks[1].end_line), (6, 8));
    }

    #[test]
    fn front_matter_is_skipped() {
        let note = "---\nsynopsis: \"x\"\nstatus: \"Done\"\n---\n\nBody text.\n";
        let chunks = chunk_markdown(note);
        assert_eq!(texts(&chunks), vec!["Body text."]);
        assert_eq!(chunks[0].start_line, 6);
    }

    #[test]
    fn long_sections_split_between_blocks_with_overlap() {
        let para = |n: usize| format!("Paragraph {n} {}. Ends here.", "word ".repeat(80));
        let note = format!("# Big\n\n{}\n\n{}\n\n{}", para(1), para(2), para(3));
        let chunks = chunk_markdown(&note);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|c| c.text.len() <= CHUNK_SIZE_LIMIT));
        assert!(chunks.iter().all(|c| c.headings == vec!["Big"]));
        assert!(chunks[0].overlap.is_empty());
        assert_eq!(chunks[1].overlap, "Ends here.");
    }

    #[test]
    fn huge_paragraph_splits_at_sentences() {
        let sentence = "The quick brown fox jumps over the lazy dog again. ";
        let note = sentence.repeat(60);
        let chunks = chunk_markdown(&note);
        assert!(chunks.len() > 1);
        for c in &chunks {
            assert!(c.text.len() <= CHUNK_SIZE_LIMIT);
            assert!(c.text.ends_with('.'), "{:?}", &c.text[c.text.len() - 10..]);
        }
    }

    #[test]
    fn unbroken_text_is_cut_on_char_boundaries() {
        let note = "é".repeat(1500);
        let chunks = chunk_markdown(&note);
        assert!(chunks.len() > 1);
        assert_eq!(chunks.iter().map(|c| c.text.as_str()).collect::<String>(), note);
    }

    #[test]
    fn empty_and_heading_only_notes() {
        assert!(chunk_markdown("").is_empty());
        assert!(chunk_markdown("\n\n  \n").is_empty());
        let chunks = chunk_markdown("# Only a title");
        assert_eq!(texts(&chunks), vec!["# Only a title"]);
        assert_eq!(chunks[0].headings, vec!["Only a title"]);
    }

    #[test]
    fn deeper_heading_then_shallower_resets_path() {
        let chunks = chunk_markdown("### Deep\n\nA\n\n# Top\n\nB");
        assert_eq!(chunks[0].headings, vec!["Deep"]);
        assert_eq!(chunks[1].headings, vec!["Top"]);
    }

    #[test]
    fn tail_takes_last_sentence_capped() {
        assert_eq!(tail("One. Two three."), "Two three.");
        assert_eq!(tail("No terminator"), "No terminator");
        let long = "x".repeat(500);
        assert_eq!(tail(&long).len(), OVERLAP_LIMIT);
    }
}
