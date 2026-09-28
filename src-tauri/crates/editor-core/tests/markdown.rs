//! Markdown in, Markdown out.
//!
//! Three properties, in order of how much they matter:
//!
//! 1. **Semantics survive.** Parsing keeps what the document says.
//! 2. **Printing is idempotent.** `parse → print → parse → print` reaches a
//!    fixed point after one pass, so a save can't keep churning a file.
//! 3. **Canonical text is untouched.** A file already in the engine's own
//!    spelling comes back byte for byte — the property that keeps an
//!    unedited save out of a Git diff.

use editor_core::{
    markdown, BlockKind, Document, DocumentSource, Editor, EditorCommand, MarkSet, SelectionRange,
};

fn parse(text: &str) -> Document {
    markdown::parse("test.md", text)
}

fn print(document: &Document) -> String {
    markdown::to_markdown(document)
}

/// Printing twice changes nothing after the first pass.
fn assert_idempotent(source: &str) -> String {
    let once = print(&parse(source));
    let twice = print(&parse(&once));
    assert_eq!(once, twice, "printing is not idempotent for:\n{source}");
    once
}

/// The document is already in the engine's spelling, so nothing moves.
fn assert_canonical(source: &str) {
    assert_eq!(
        print(&parse(source)),
        source,
        "canonical text was rewritten"
    );
}

// --- blocks -----------------------------------------------------------------

#[test]
fn parses_headings_and_paragraphs() {
    let document = parse("# Title\n\nSome prose.\n\n### Deeper\n");
    let blocks = document.blocks();
    assert_eq!(blocks.len(), 3);
    match &blocks[0].kind {
        BlockKind::Heading { level, content } => {
            assert_eq!(*level, 1);
            assert_eq!(content.plain_text(), "Title");
        }
        other => panic!("expected a heading, got {}", other.name()),
    }
    assert_eq!(blocks[1].kind.name(), "paragraph");
    assert_eq!(document.outline().len(), 2);
    assert_canonical("# Title\n\nSome prose.\n\n### Deeper\n");
}

#[test]
fn parses_marks_and_keeps_them_apart() {
    let document = parse("Plain **bold** and _italic_ and ~~gone~~ and `code`.\n");
    let content = document.blocks()[0].content().unwrap();
    assert_eq!(
        content.plain_text(),
        "Plain bold and italic and gone and code."
    );
    assert!(content.has_mark_throughout(6, 10, MarkSet::BOLD));
    assert!(content.has_mark_throughout(15, 21, MarkSet::ITALIC));
    assert!(content.has_mark_throughout(26, 30, MarkSet::STRIKE));
    assert!(content.has_mark_throughout(35, 39, MarkSet::CODE));
    assert_canonical("Plain **bold** and _italic_ and ~~gone~~ and `code`.\n");
}

#[test]
fn nested_markup_flattens_onto_runs() {
    // Nesting is a spelling; what comes back is a run carrying both marks.
    let document = parse("**bold _and italic_**\n");
    let content = document.blocks()[0].content().unwrap();
    assert_eq!(content.plain_text(), "bold and italic");
    assert!(content.has_mark_throughout(0, 5, MarkSet::BOLD));
    assert!(content.has_mark_throughout(5, 15, MarkSet::BOLD));
    assert!(content.has_mark_throughout(5, 15, MarkSet::ITALIC));
    assert!(!content.has_mark_throughout(0, 15, MarkSet::ITALIC));

    // And the marks, not the spelling, are what survives the round trip.
    let again = parse(&assert_idempotent("**bold _and italic_**\n"));
    let content = again.blocks()[0].content().unwrap();
    assert_eq!(content.plain_text(), "bold and italic");
    assert!(content.has_mark_throughout(5, 15, MarkSet::ITALIC));
    assert!(content.has_mark_throughout(0, 15, MarkSet::BOLD));
}

#[test]
fn emphasis_never_closes_against_a_space() {
    // `**bold **` is not strong in CommonMark, so the space moves outside.
    let document = parse("**bold ** trailing\n");
    let printed = print(&document);
    let reparsed = parse(&printed);
    assert_eq!(
        reparsed.blocks()[0].content().unwrap().plain_text(),
        document.blocks()[0].content().unwrap().plain_text()
    );

    let spaced = parse("a **b** c\n");
    let content = spaced.blocks()[0].content().unwrap();
    let printed = print(&spaced);
    assert!(printed.contains("**b**"), "got {printed:?}");
    assert!(content.has_mark_throughout(2, 3, MarkSet::BOLD));
}

#[test]
fn parses_links_and_images() {
    let document = parse("See [the notes](notes.md \"Notes\") and ![a map](map.png).\n");
    let content = document.blocks()[0].content().unwrap();
    let link = content.link_at(5).expect("a link");
    assert_eq!(link.url, "notes.md");
    assert_eq!(link.title.as_deref(), Some("Notes"));
    assert!(
        content.plain_text().contains("a map"),
        "alt text is the image's text"
    );
    assert_canonical("See [the notes](notes.md \"Notes\") and ![a map](map.png).\n");
}

#[test]
fn urls_with_spaces_are_bracketed() {
    let printed = assert_idempotent("[x](<a file.md>)\n");
    assert!(printed.contains("<a file.md>"), "got {printed:?}");
    assert_eq!(
        parse(&printed).blocks()[0]
            .content()
            .unwrap()
            .link_at(0)
            .unwrap()
            .url,
        "a file.md"
    );
}

#[test]
fn parses_code_blocks_with_their_language() {
    let document = parse("```rust\nfn main() {}\n```\n");
    match &document.blocks()[0].kind {
        BlockKind::CodeBlock { language, code } => {
            assert_eq!(language.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}");
        }
        other => panic!("expected a code block, got {}", other.name()),
    }
    assert_canonical("```rust\nfn main() {}\n```\n");
}

#[test]
fn a_code_block_holding_a_fence_gets_a_longer_one() {
    let source = "````\n```\nnot a fence\n```\n````\n";
    let document = parse(source);
    match &document.blocks()[0].kind {
        BlockKind::CodeBlock { code, .. } => assert_eq!(code, "```\nnot a fence\n```"),
        other => panic!("expected a code block, got {}", other.name()),
    }
    assert_canonical(source);
}

#[test]
fn parses_quotes_including_nested_blocks() {
    let document = parse("> Quoted prose.\n>\n> ## And a heading\n");
    match &document.blocks()[0].kind {
        BlockKind::Quote { blocks } => {
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[1].kind.name(), "heading");
        }
        other => panic!("expected a quote, got {}", other.name()),
    }
    assert_eq!(document.plain_text(), "Quoted prose.\n\nAnd a heading");
    assert_canonical("> Quoted prose.\n>\n> ## And a heading\n");
}

#[test]
fn parses_lists_tight_and_loose() {
    let tight = parse("- one\n- two\n");
    match &tight.blocks()[0].kind {
        BlockKind::List {
            ordered,
            tight,
            items,
            ..
        } => {
            assert!(!ordered);
            assert!(tight);
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected a list, got {}", other.name()),
    }
    assert_canonical("- one\n- two\n");

    let loose = parse("- one\n\n- two\n");
    match &loose.blocks()[0].kind {
        BlockKind::List { tight, .. } => assert!(!tight, "blank lines mean a loose list"),
        other => panic!("expected a list, got {}", other.name()),
    }
    assert_canonical("- one\n\n- two\n");
}

#[test]
fn parses_ordered_lists_keeping_their_first_number() {
    let document = parse("3. three\n4. four\n");
    match &document.blocks()[0].kind {
        BlockKind::List { ordered, start, .. } => {
            assert!(ordered);
            assert_eq!(*start, Some(3));
        }
        other => panic!("expected a list, got {}", other.name()),
    }
    assert_canonical("3. three\n4. four\n");
    assert_canonical("1. one\n2. two\n");
}

#[test]
fn parses_nested_lists() {
    let source = "- one\n  - inner\n  - also inner\n- two\n";
    let document = parse(source);
    let BlockKind::List { items, .. } = &document.blocks()[0].kind else {
        panic!("expected a list");
    };
    assert_eq!(
        items[0].blocks.len(),
        2,
        "the item's text, then the nested list"
    );
    assert_eq!(items[0].blocks[1].kind.name(), "list");
    assert_eq!(document.plain_text(), "one\n\ninner\n\nalso inner\n\ntwo");
    assert_canonical(source);
}

#[test]
fn parses_task_lists() {
    let source = "- [x] done\n- [ ] not done\n";
    let document = parse(source);
    let BlockKind::List { items, .. } = &document.blocks()[0].kind else {
        panic!("expected a list");
    };
    assert_eq!(items[0].checked, Some(true));
    assert_eq!(items[1].checked, Some(false));
    assert_canonical(source);
}

#[test]
fn parses_tables_with_alignment() {
    let source = "| Name | Role |\n| :--- | ---: |\n| Ada | Writer |\n";
    let document = parse(source);
    let BlockKind::Table { table } = &document.blocks()[0].kind else {
        panic!("expected a table");
    };
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0].cells.len(), 2);
    assert_eq!(
        table.rows[1].cells[0].blocks[0]
            .content()
            .unwrap()
            .plain_text(),
        "Ada"
    );
    assert_eq!(table.alignments[0], editor_core::ColumnAlignment::Left);
    assert_eq!(table.alignments[1], editor_core::ColumnAlignment::Right);
    assert_canonical(source);
}

#[test]
fn parses_thematic_breaks_without_being_mistaken_for_front_matter() {
    let document = parse("---\n\nafter\n");
    assert_eq!(document.blocks()[0].kind.name(), "thematicBreak");
    assert!(document.metadata.frontmatter.is_none());
    // A rule that opens a document is printed as `***`, which can't be read
    // back as front matter.
    let printed = print(&document);
    assert!(printed.starts_with("***"), "got {printed:?}");
    assert_eq!(parse(&printed).blocks()[0].kind.name(), "thematicBreak");
    assert_idempotent("---\n\nafter\n");

    // Not at the start, `---` is unambiguous and is kept.
    assert_canonical("before\n\n---\n\nafter\n");
}

#[test]
fn keeps_raw_html() {
    let block = parse("<div class=\"note\">\nhi\n</div>\n");
    assert_eq!(block.blocks()[0].kind.name(), "html");
    assert_canonical("<div class=\"note\">\nhi\n</div>\n");

    let inline = parse("a <br> b\n");
    assert_eq!(inline.blocks()[0].kind.name(), "paragraph");
    assert!(
        print(&inline).contains("<br>"),
        "inline HTML is kept verbatim"
    );
    assert_idempotent("a <br> b\n");
}

#[test]
fn keeps_hard_and_soft_breaks() {
    // A hard break stays a hard break…
    let hard = parse("one\\\ntwo\n");
    assert_eq!(hard.blocks()[0].content().unwrap().plain_text(), "one\ntwo");
    assert_canonical("one\\\ntwo\n");

    // …and the author's own wrapping is not reflowed into one long line.
    assert_canonical("a line\nand its continuation\n");
}

#[test]
fn escapes_text_that_would_otherwise_be_markup() {
    for source in [
        "a literal \\*star\\*\n",
        "a \\_score\\_\n",
        "\\# not a heading\n",
        "\\- not a list\n",
        "1\\. not a list\n",
        "\\> not a quote\n",
        "a \\[bracket\\]\n",
        "back\\\\slash\n",
    ] {
        let document = parse(source);
        let printed = print(&document);
        assert_eq!(
            parse(&printed).plain_text(),
            document.plain_text(),
            "escaping changed the text of {source:?} (printed {printed:?})"
        );
        assert_idempotent(source);
    }

    // The escape is gone from the model: it is text, not markup.
    assert_eq!(parse("a \\*star\\*\n").plain_text(), "a *star*");
}

#[test]
fn inline_code_holding_backticks_gets_a_longer_fence() {
    let document = parse("use ``a ` b`` here\n");
    let content = document.blocks()[0].content().unwrap();
    assert_eq!(content.plain_text(), "use a ` b here");
    assert!(content.has_mark_throughout(4, 9, MarkSet::CODE));
    assert_idempotent("use ``a ` b`` here\n");
}

// --- front matter -----------------------------------------------------------

#[test]
fn front_matter_is_kept_out_of_the_blocks_and_written_back_verbatim() {
    let source = "---\nsynopsis: \"She finds the letter.\"\nstatus: 'First Draft'\ntags:\n  - one\n  - two\n---\n\n# Scene\n";
    let document = parse(source);

    let frontmatter = document
        .metadata
        .frontmatter
        .as_ref()
        .expect("front matter");
    assert_eq!(frontmatter.fields["synopsis"], "She finds the letter.");
    assert_eq!(frontmatter.fields["status"], "First Draft");
    assert_eq!(document.blocks().len(), 1, "metadata is not a block");
    assert_eq!(document.blocks()[0].kind.name(), "heading");

    // Byte for byte, including the YAML this app doesn't understand.
    assert_eq!(print(&document), source);
}

#[test]
fn front_matter_block_scalars_are_read() {
    let document = parse("---\nsynopsis: |\n  first line\n  second line\nstatus: >\n  folded\n  onto one line\n---\n\nbody\n");
    let fields = &document.metadata.frontmatter.as_ref().unwrap().fields;
    assert_eq!(fields["synopsis"], "first line\nsecond line");
    assert_eq!(fields["status"], "folded onto one line");
}

#[test]
fn a_leading_rule_is_not_front_matter() {
    let document = parse("---\nnot: yaml because this line has no key\n---\n");
    assert!(document.metadata.frontmatter.is_none() || document.blocks().len() == 1);

    // A `---` fence with prose inside is a heading-ish rule, not metadata.
    let rules = parse("---\njust prose\n---\n");
    assert!(rules.metadata.frontmatter.is_none());
}

#[test]
fn generated_front_matter_quotes_its_values() {
    let mut document = parse("body\n");
    document.metadata.frontmatter = Some(editor_core::Frontmatter {
        raw: String::new(),
        fields: [(
            "synopsis".to_string(),
            "She said \"no\".\nThen left.".to_string(),
        )]
        .into_iter()
        .collect(),
    });

    let printed = print(&document);
    assert!(
        printed.starts_with("---\nsynopsis: \"She said \\\"no\\\".\\nThen left.\"\n---\n\n"),
        "got {printed:?}"
    );
    let back = parse(&printed);
    assert_eq!(
        back.metadata.frontmatter.unwrap().fields["synopsis"],
        "She said \"no\".\nThen left."
    );
}

// --- fixtures ---------------------------------------------------------------

#[test]
fn fixture_documents_round_trip() {
    for (name, source) in [
        ("kitchen-sink.md", include_str!("fixtures/kitchen-sink.md")),
        ("scene.md", include_str!("fixtures/scene.md")),
    ] {
        let document = parse(source);
        let printed = print(&document);
        assert_eq!(printed, source, "{name} was rewritten by a save");

        // And editing it doesn't disturb anything but the block edited.
        let mut editor = Editor::new(parse(source));
        let first = editor
            .document()
            .text_block_ids()
            .first()
            .copied()
            .expect("a text block");
        editor
            .apply(EditorCommand::SetSelection(SelectionRange::in_block(
                first, 0, 0,
            )))
            .unwrap();
        editor
            .apply(EditorCommand::insert_text("Edited: "))
            .unwrap();
        let edited = editor.document().markdown();
        assert!(
            edited.contains("Edited: "),
            "{name}: the edit is in the output"
        );
        assert_eq!(
            edited.lines().count(),
            source.lines().count(),
            "{name}: an inline edit changed the line layout"
        );

        editor.apply(EditorCommand::Undo).unwrap();
        assert_eq!(
            editor.document().markdown(),
            source,
            "{name}: undo restored the file"
        );
    }
}

#[test]
fn document_source_gives_text_and_markdown() {
    let document = parse("# Title\n\nSome **prose**.\n");
    assert_eq!(document.plain_text(), "Title\n\nSome prose.");
    assert_eq!(document.markdown(), "# Title\n\nSome **prose**.\n");
}

#[test]
fn other_spellings_reach_a_fixed_point_without_losing_text() {
    // Markdown says the same thing many ways. The engine keeps the meaning and
    // settles on one spelling after a single pass — it never keeps churning.
    for source in [
        "*emphasis* and __strong__\n",
        "Setext title\n============\n\nbody\n",
        "+ plus item\n+ another\n",
        "1) paren ordered\n2) again\n",
        "***\n\nafter a starred rule\n",
        "> - a list inside a quote\n> - second\n",
        "- - deeply nested single item\n",
        "1. loose\n\n   with a second paragraph\n\n2. next\n",
        "[reference][ref]\n\n[ref]: notes.md\n",
        "<https://example.com>\n",
        "an &amp; entity\n",
        "trailing spaces for a break  \nnext line\n",
        "| a |\n| --- |\n| **bold** cell |\n",
        "Ünïcödé — em dash, 日本語, 🙂\n",
        "\tan indented code block\n",
        "# heading with `code` and [a link](x.md)\n",
        "![](empty-alt.png)\n",
        "text with a | pipe outside a table\n",
    ] {
        let before = parse(source);
        let printed = assert_idempotent(source);
        let after = parse(&printed);
        assert_eq!(
            after.plain_text(),
            before.plain_text(),
            "text changed for {source:?} (printed {printed:?})"
        );
        assert_eq!(
            after
                .blocks()
                .iter()
                .map(|b| b.kind.name())
                .collect::<Vec<_>>(),
            before
                .blocks()
                .iter()
                .map(|b| b.kind.name())
                .collect::<Vec<_>>(),
            "structure changed for {source:?} (printed {printed:?})"
        );
    }
}
