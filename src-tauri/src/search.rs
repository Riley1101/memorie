//! Find and replace across every writing in the content directory.

use regex::{NoExpand, Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

/// Stop collecting past this many matches so a one-letter query stays fast.
const MAX_MATCHES: usize = 2000;
/// Characters of context kept on each side of a match in previews.
const PREVIEW_CONTEXT: usize = 60;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub whole_word: bool,
    #[serde(default)]
    pub regex: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    /// 1-based line number.
    pub line: usize,
    /// The line, trimmed to some context around the match.
    pub preview: String,
    /// Match bounds within `preview`, in UTF-16 code units so JS can slice them.
    pub start: usize,
    pub end: usize,
    /// In the front matter (synopsis, status…) rather than the text itself.
    pub meta: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMatches {
    pub name: String,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub files: Vec<FileMatches>,
    pub total: usize,
    /// True when `MAX_MATCHES` cut the search short.
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Replaced {
    pub name: String,
    pub count: usize,
}

pub fn build_regex(query: &str, options: &SearchOptions) -> Result<Regex, String> {
    let pattern = if options.regex { query.to_string() } else { regex::escape(query) };
    let pattern = if options.whole_word { format!(r"\b(?:{pattern})\b") } else { pattern };
    RegexBuilder::new(&pattern)
        .case_insensitive(!options.case_sensitive)
        .multi_line(true)
        .build()
        .map_err(|e| format!("Invalid pattern: {e}"))
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Shortens `line` around the byte range `start..end` and returns the preview
/// with the match bounds re-expressed in UTF-16 units.
fn preview(line: &str, start: usize, end: usize) -> SearchMatch {
    let mut from = start;
    for (chars_back, (i, _)) in line[..start].char_indices().rev().enumerate() {
        if chars_back == PREVIEW_CONTEXT {
            break;
        }
        from = i;
    }
    let mut to = end;
    for (count, (i, c)) in line[end..].char_indices().enumerate() {
        if count == PREVIEW_CONTEXT {
            break;
        }
        to = end + i + c.len_utf8();
    }

    let prefix = if from > 0 { "…" } else { "" };
    let suffix = if to < line.len() { "…" } else { "" };
    let before = format!("{prefix}{}", &line[from..start]);
    let text = format!("{before}{}{}{suffix}", &line[start..end], &line[end..to]);
    let start16 = utf16_len(&before);
    SearchMatch { line: 0, preview: text, start: start16, end: start16 + utf16_len(&line[start..end]), meta: false }
}

/// Searches already-loaded contents. Split out from the command for testing.
pub fn search_contents(regex: &Regex, contents: Vec<(String, String)>) -> SearchResults {
    let mut files = Vec::new();
    let mut total = 0;
    let mut truncated = false;

    'files: for (name, content) in contents {
        let mut matches = Vec::new();
        let meta_lines = front_matter_lines(&content);
        for (index, line) in content.lines().enumerate() {
            for m in regex.find_iter(line) {
                if m.start() == m.end() {
                    continue;
                }
                if total >= MAX_MATCHES {
                    truncated = true;
                    if !matches.is_empty() {
                        files.push(FileMatches { name, matches });
                    }
                    break 'files;
                }
                let mut found = preview(line, m.start(), m.end());
                found.line = index + 1;
                found.meta = index < meta_lines;
                matches.push(found);
                total += 1;
            }
        }
        if !matches.is_empty() {
            files.push(FileMatches { name, matches });
        }
    }

    SearchResults { files, total, truncated }
}

/// Number of lines (including both `---`) taken by front matter at the top,
/// or 0. Same rule as the export: every line inside must read like YAML.
fn front_matter_lines(content: &str) -> usize {
    let stripped = crate::export::strip_front_matter(content);
    if stripped.len() == content.len() {
        return 0;
    }
    content[..content.len() - stripped.len()].lines().count()
}

/// Applies a replacement to one file's content, line by line so it changes
/// exactly what the search listed. `$1`-style references are only honoured in
/// regex mode, so a literal "$5" in plain mode stays "$5".
pub fn replace_content(regex: &Regex, content: &str, replacement: &str, options: &SearchOptions) -> (String, usize) {
    let mut out = String::with_capacity(content.len());
    let mut count = 0;
    for line in content.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let ending = &line[body.len()..];
        let n = regex.find_iter(body).filter(|m| m.start() != m.end()).count();
        if n == 0 {
            out.push_str(line);
            continue;
        }
        count += n;
        let replaced = if options.regex {
            regex.replace_all(body, replacement)
        } else {
            regex.replace_all(body, NoExpand(replacement))
        };
        out.push_str(&replaced);
        out.push_str(ending);
    }
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> SearchOptions {
        SearchOptions::default()
    }

    #[test]
    fn finds_case_insensitively_with_utf16_offsets() {
        let re = build_regex("tavern", &opts()).unwrap();
        let results = search_contents(
            &re,
            vec![("a.md".into(), "Intro\n😀 The Tavern was loud. tavern!".into())],
        );
        assert_eq!(results.total, 2);
        let first = &results.files[0].matches[0];
        assert_eq!(first.line, 2);
        let units: Vec<u16> = first.preview.encode_utf16().collect();
        assert_eq!(String::from_utf16(&units[first.start..first.end]).unwrap(), "Tavern");
    }

    #[test]
    fn whole_word_and_case_sensitive() {
        let options = SearchOptions { case_sensitive: true, whole_word: true, regex: false };
        let re = build_regex("Ann", &options).unwrap();
        let results = search_contents(&re, vec![("a.md".into(), "Ann met Anna and ann.".into())]);
        assert_eq!(results.total, 1);
    }

    #[test]
    fn plain_mode_escapes_and_does_not_expand() {
        let re = build_regex("$5 (cash)", &opts()).unwrap();
        let (out, n) = replace_content(&re, "Pay $5 (cash) now", "$1 card", &opts());
        assert_eq!(n, 1);
        assert_eq!(out, "Pay $1 card now");
    }

    #[test]
    fn regex_mode_expands_groups() {
        let options = SearchOptions { regex: true, ..opts() };
        let re = build_regex(r"(\w+) Smith", &options).unwrap();
        let (out, n) = replace_content(&re, "John Smith and Jane Smith", "$1 Jones", &options);
        assert_eq!(n, 2);
        assert_eq!(out, "John Jones and Jane Jones");
    }

    #[test]
    fn replace_never_crosses_lines_and_keeps_endings() {
        let options = SearchOptions { regex: true, ..opts() };
        let re = build_regex(r"a\s+b", &options).unwrap();
        let (out, n) = replace_content(&re, "a b\r\na\nb\n", "X", &options);
        assert_eq!(n, 1);
        assert_eq!(out, "X\r\na\nb\n");
    }

    #[test]
    fn front_matter_matches_are_marked() {
        let re = build_regex("ann", &opts()).unwrap();
        let results = search_contents(&re, vec![("a.md".into(), "---\nsynopsis: \"Ann leaves\"\n---\n\nAnn stays.".into())]);
        let flags: Vec<bool> = results.files[0].matches.iter().map(|m| m.meta).collect();
        assert_eq!(flags, vec![true, false]);
    }

    #[test]
    fn invalid_regex_is_an_error() {
        let options = SearchOptions { regex: true, ..opts() };
        assert!(build_regex("(unclosed", &options).is_err());
    }

    #[test]
    fn long_lines_are_trimmed_around_the_match() {
        let line = format!("{}needle{}", "a".repeat(500), "b".repeat(500));
        let re = build_regex("needle", &opts()).unwrap();
        let results = search_contents(&re, vec![("a.md".into(), line)]);
        let m = &results.files[0].matches[0];
        assert!(m.preview.chars().count() < 140);
        assert!(m.preview.starts_with('…') && m.preview.ends_with('…'));
        let units: Vec<u16> = m.preview.encode_utf16().collect();
        assert_eq!(String::from_utf16(&units[m.start..m.end]).unwrap(), "needle");
    }
}
