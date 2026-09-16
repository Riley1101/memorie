//! A small RTF reader: enough to bring prose (paragraphs, bold, italic,
//! Unicode) out of Scrivener projects and plain `.rtf` files as Markdown.
//! Tables, images, fonts and colours are ignored.

/// Destinations whose content is never body text.
const SKIPPED_DESTINATIONS: &[&str] = &[
    "fonttbl", "colortbl", "stylesheet", "info", "pict", "header", "headerl", "headerr", "headerf",
    "footer", "footerl", "footerr", "footerf", "footnote", "listtable", "listoverridetable",
    "generator", "xmlnstbl", "rsidtbl", "themedata", "colorschememapping", "latentstyles",
    "datastore", "mmathPr", "pgdsctbl", "object", "fldinst", "field_instructions", "revtbl",
    "filetbl", "private", "nonshppict", "shp", "expandedcolortbl", "annotation", "atnid",
    "atnauthor", "comment",
];

#[derive(Clone, Copy)]
struct GroupState {
    bold: bool,
    italic: bool,
    skip: bool,
    /// Fallback characters to drop after a `\uN` escape.
    uc: usize,
}

impl Default for GroupState {
    fn default() -> Self {
        GroupState { bold: false, italic: false, skip: false, uc: 1 }
    }
}

struct Paragraphs {
    done: Vec<String>,
    runs: Vec<(String, bool, bool)>,
}

impl Paragraphs {
    fn push_char(&mut self, c: char, state: &GroupState) {
        match self.runs.last_mut() {
            Some((text, bold, italic)) if *bold == state.bold && *italic == state.italic => text.push(c),
            _ => self.runs.push((c.to_string(), state.bold, state.italic)),
        }
    }

    fn end_paragraph(&mut self) {
        let runs = std::mem::take(&mut self.runs);
        let mut line = String::new();
        for (text, bold, italic) in runs {
            line.push_str(&render_run(&text, bold, italic));
        }
        let line = line.trim_end().to_string();
        self.done.push(escape_line_start(&line));
    }

    fn finish(mut self) -> String {
        if !self.runs.is_empty() {
            self.end_paragraph();
        }
        // Blank paragraphs are spacing in RTF; in Markdown one blank line is enough.
        self.done
            .into_iter()
            .filter(|p| !p.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

fn escape_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '[' | ']' | '`' | '<') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Markdown reads some line starts as syntax ("# ", "> ", "- ", "1. ").
fn escape_line_start(line: &str) -> String {
    let trimmed = line.trim_start();
    let needs = trimmed.starts_with('#')
        || trimmed.starts_with('>')
        || trimmed.starts_with("- ")
        || trimmed.starts_with("+ ")
        || {
            let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
            !digits.is_empty() && trimmed[digits.len()..].starts_with(". ")
        };
    if !needs {
        return trimmed.to_string();
    }
    if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        let digits = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
        format!("{}\\{}", &trimmed[..digits], &trimmed[digits..])
    } else {
        format!("\\{trimmed}")
    }
}

/// Emphasis markers must hug the text, so surrounding spaces move outside them.
fn render_run(text: &str, bold: bool, italic: bool) -> String {
    let escaped = escape_inline(text);
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

/// Windows-1252 differs from Latin-1 only in 0x80..=0x9F.
fn cp1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
        '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    match byte {
        0x80..=0x9F => HIGH[(byte - 0x80) as usize],
        _ => byte as char,
    }
}

/// Converts RTF to Markdown paragraphs.
pub fn rtf_to_markdown(rtf: &str) -> String {
    let bytes = rtf.as_bytes();
    let mut stack: Vec<GroupState> = Vec::new();
    let mut state = GroupState::default();
    let mut out = Paragraphs { done: Vec::new(), runs: Vec::new() };
    // After `\uN`, this many following characters are the ANSI fallback.
    let mut pending_skip = 0usize;
    // Set right after `{` so we can recognise `{\*\dest ...}` and `{\fonttbl ...}`.
    let mut group_start = false;
    let mut i = 0;

    let emit = |c: char, state: &GroupState, pending_skip: &mut usize, out: &mut Paragraphs| {
        if state.skip {
            return;
        }
        if *pending_skip > 0 {
            *pending_skip -= 1;
            return;
        }
        out.push_char(c, state);
    };

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'{' => {
                stack.push(state);
                group_start = true;
                i += 1;
                continue;
            }
            b'}' => {
                state = stack.pop().unwrap_or_default();
                pending_skip = 0;
                i += 1;
            }
            b'\\' => {
                i += 1;
                let Some(&next) = bytes.get(i) else { break };
                if next.is_ascii_alphabetic() {
                    let start = i;
                    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word = &rtf[start..i];
                    let mut param: Option<i32> = None;
                    let num_start = i;
                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                        i += 1;
                        while i < bytes.len() && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                        param = rtf[num_start..i].parse().ok();
                    }
                    // A single space after a control word is part of it.
                    if i < bytes.len() && bytes[i] == b' ' {
                        i += 1;
                    }

                    if group_start && SKIPPED_DESTINATIONS.contains(&word) {
                        state.skip = true;
                    }
                    match word {
                        "par" | "sect" | "page" => {
                            if !state.skip {
                                out.end_paragraph();
                            }
                        }
                        "line" => emit('\n', &state, &mut pending_skip, &mut out),
                        "tab" => emit('\t', &state, &mut pending_skip, &mut out),
                        "b" => state.bold = param != Some(0),
                        "i" => state.italic = param != Some(0),
                        "plain" => {
                            state.bold = false;
                            state.italic = false;
                        }
                        "uc" => state.uc = param.unwrap_or(1).max(0) as usize,
                        "u" => {
                            if let Some(n) = param {
                                let code = if n < 0 { (n + 65536) as u32 } else { n as u32 };
                                if let Some(c) = char::from_u32(code) {
                                    emit(c, &state, &mut 0, &mut out);
                                }
                                pending_skip = state.uc;
                            }
                        }
                        "emdash" => emit('—', &state, &mut pending_skip, &mut out),
                        "endash" => emit('–', &state, &mut pending_skip, &mut out),
                        "lquote" => emit('‘', &state, &mut pending_skip, &mut out),
                        "rquote" => emit('’', &state, &mut pending_skip, &mut out),
                        "ldblquote" => emit('“', &state, &mut pending_skip, &mut out),
                        "rdblquote" => emit('”', &state, &mut pending_skip, &mut out),
                        "bullet" => emit('•', &state, &mut pending_skip, &mut out),
                        _ => {}
                    }
                } else {
                    i += 1;
                    match next {
                        b'*' => state.skip = true,
                        b'\'' => {
                            if let Some(hex) = rtf.get(i..i + 2) {
                                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                                    emit(cp1252(byte), &state, &mut pending_skip, &mut out);
                                }
                                i += 2;
                            }
                        }
                        b'~' => emit('\u{a0}', &state, &mut pending_skip, &mut out),
                        b'_' => emit('‑', &state, &mut pending_skip, &mut out),
                        b'-' => {}
                        b'\n' | b'\r' => {
                            if !state.skip {
                                out.end_paragraph();
                            }
                        }
                        other => emit(other as char, &state, &mut pending_skip, &mut out),
                    }
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                // Plain text: take the whole UTF-8 character (RTF should be ASCII, but be kind).
                let ch = rtf[i..].chars().next().unwrap_or('\u{fffd}');
                emit(ch, &state, &mut pending_skip, &mut out);
                i += ch.len_utf8();
            }
        }
        group_start = false;
    }

    out.finish()
}

#[cfg(test)]
mod tests {
    use super::rtf_to_markdown;

    #[test]
    fn paragraphs_and_emphasis() {
        let rtf = r"{\rtf1\ansi\ansicpg1252{\fonttbl\f0\fnil Palatino;}{\colortbl;\red0\green0\blue0;}
\pard\f0\fs24 It was a {\b dark} and {\i stormy} night.\par
\par
She said \ldblquote hi\rdblquote  \emdash  then left.\par}";
        assert_eq!(
            rtf_to_markdown(rtf),
            "It was a **dark** and *stormy* night.\n\nShe said “hi” — then left."
        );
    }

    #[test]
    fn unicode_hex_and_ignorable_destinations() {
        let rtf = r"{\rtf1{\*\generator Scrivener;}caf\'e9 \u8364?5 na\uc0\u239 ve\par}";
        assert_eq!(rtf_to_markdown(rtf), "café €5 naïve");
    }

    #[test]
    fn toggles_and_escapes() {
        let rtf = r"{\rtf1 \b bold\b0  plain *star* \{brace\}\par # not heading\par}";
        assert_eq!(rtf_to_markdown(rtf), "**bold** plain \\*star\\* {brace}\n\n\\# not heading");
    }
}
