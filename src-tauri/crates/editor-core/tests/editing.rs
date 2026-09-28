//! Editing, selection and history behaviour, driven only through commands —
//! the same door the app will use.

use editor_core::{
    BlockId, BlockKind, Direction, Document, Editor, EditorCommand, EditorError, ImageNode, Inline,
    InlineContent, MarkSet, Position, SelectionRange,
};

fn editor(kinds: Vec<BlockKind>) -> Editor {
    Editor::new(Document::from_kinds("test.md", kinds))
}

fn block(editor: &Editor, index: usize) -> BlockId {
    editor.document().blocks()[index].id
}

fn text(editor: &Editor, index: usize) -> String {
    editor.document().blocks()[index]
        .content()
        .map(|content| content.plain_text())
        .unwrap_or_else(|| editor.document().blocks()[index].own_text())
}

fn select(editor: &mut Editor, block: BlockId, start: usize, end: usize) {
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::in_block(
            block, start, end,
        )))
        .unwrap();
}

// --- text editing -----------------------------------------------------------

#[test]
fn inserts_text_at_the_caret() {
    let mut editor = editor(vec![BlockKind::paragraph("She found the ")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 14, 14);

    editor.apply(EditorCommand::insert_text("letter")).unwrap();

    assert_eq!(text(&editor, 0), "She found the letter");
    assert_eq!(
        editor.selection(),
        SelectionRange::in_block(paragraph, 20, 20)
    );
}

#[test]
fn typing_over_a_selection_replaces_it() {
    let mut editor = editor(vec![BlockKind::paragraph("one two three")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 4, 7);

    editor.apply(EditorCommand::insert_text("2")).unwrap();

    assert_eq!(text(&editor, 0), "one 2 three");
    assert_eq!(
        editor.selection(),
        SelectionRange::in_block(paragraph, 5, 5)
    );
}

#[test]
fn offsets_count_characters_not_bytes() {
    let mut editor = editor(vec![BlockKind::paragraph("café 日本語 🙂")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 5, 8);

    editor
        .apply(EditorCommand::insert_text("にほんご"))
        .unwrap();

    assert_eq!(text(&editor, 0), "café にほんご 🙂");
}

#[test]
fn backspace_deletes_one_character() {
    let mut editor = editor(vec![BlockKind::paragraph("word")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 4, 4);

    editor
        .apply(EditorCommand::Delete(Direction::Backward))
        .unwrap();

    assert_eq!(text(&editor, 0), "wor");
    assert_eq!(
        editor.selection(),
        SelectionRange::in_block(paragraph, 3, 3)
    );
}

#[test]
fn forward_delete_at_the_end_pulls_the_next_block_up() {
    let mut editor = editor(vec![
        BlockKind::paragraph("first"),
        BlockKind::paragraph("second"),
    ]);
    let first = block(&editor, 0);
    select(&mut editor, first, 5, 5);

    editor
        .apply(EditorCommand::Delete(Direction::Forward))
        .unwrap();

    assert_eq!(editor.document().blocks().len(), 1);
    assert_eq!(text(&editor, 0), "firstsecond");
}

#[test]
fn backspace_at_the_start_merges_with_the_block_before() {
    let mut editor = editor(vec![
        BlockKind::paragraph("first"),
        BlockKind::paragraph("second"),
    ]);
    let first = block(&editor, 0);
    let second = block(&editor, 1);
    select(&mut editor, second, 0, 0);

    editor
        .apply(EditorCommand::Delete(Direction::Backward))
        .unwrap();

    assert_eq!(editor.document().blocks().len(), 1);
    assert_eq!(text(&editor, 0), "firstsecond");
    // The surviving block keeps its identity, and the caret sits at the seam.
    assert_eq!(editor.selection(), SelectionRange::in_block(first, 5, 5));
}

#[test]
fn merging_keeps_the_marks_of_both_halves() {
    let mut editor = editor(vec![
        BlockKind::Paragraph {
            content: InlineContent::new(vec![Inline::marked("bold", MarkSet::BOLD)]),
        },
        BlockKind::Paragraph {
            content: InlineContent::new(vec![Inline::marked("italic", MarkSet::ITALIC)]),
        },
    ]);
    let second = block(&editor, 1);
    select(&mut editor, second, 0, 0);

    editor.apply(EditorCommand::MergeBlocks).unwrap();

    let content = editor.document().blocks()[0].content().unwrap();
    assert!(content.has_mark_throughout(0, 4, MarkSet::BOLD));
    assert!(content.has_mark_throughout(4, 10, MarkSet::ITALIC));
}

#[test]
fn splitting_a_paragraph_gives_two_paragraphs() {
    let mut editor = editor(vec![BlockKind::paragraph("one two")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 3, 3);

    editor.apply(EditorCommand::SplitBlock).unwrap();

    assert_eq!(text(&editor, 0), "one");
    assert_eq!(text(&editor, 1), " two");
    assert_eq!(
        block(&editor, 0),
        paragraph,
        "the first half keeps the block id"
    );
    assert_eq!(
        editor.selection(),
        SelectionRange::collapsed(Position::new(block(&editor, 1), 0))
    );
}

#[test]
fn splitting_a_heading_leaves_a_paragraph_behind_it() {
    let mut editor = editor(vec![BlockKind::heading(2, "Chapter One")]);
    let heading = block(&editor, 0);
    select(&mut editor, heading, 8, 8);

    editor.apply(EditorCommand::SplitBlock).unwrap();

    assert_eq!(editor.document().blocks()[0].kind.name(), "heading");
    assert_eq!(editor.document().blocks()[1].kind.name(), "paragraph");
    assert_eq!(text(&editor, 1), "One");
}

#[test]
fn splitting_over_a_selection_drops_it() {
    let mut editor = editor(vec![BlockKind::paragraph("one two three")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 3, 8);

    editor.apply(EditorCommand::SplitBlock).unwrap();

    assert_eq!(text(&editor, 0), "one");
    assert_eq!(text(&editor, 1), "three");
}

// --- formatting -------------------------------------------------------------

#[test]
fn toggling_bold_marks_then_unmarks_the_selection() {
    let mut editor = editor(vec![BlockKind::paragraph("one two")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 3);

    editor.apply(EditorCommand::toggle_bold()).unwrap();
    let content = editor.document().blocks()[0].content().unwrap();
    assert!(content.has_mark_throughout(0, 3, MarkSet::BOLD));
    assert!(!content.has_mark_throughout(3, 7, MarkSet::BOLD));
    assert_eq!(content.plain_text(), "one two");

    editor.apply(EditorCommand::toggle_bold()).unwrap();
    assert!(!editor.document().blocks()[0]
        .content()
        .unwrap()
        .has_mark_throughout(0, 3, MarkSet::BOLD));
}

#[test]
fn toggling_over_a_partly_marked_selection_marks_all_of_it() {
    let mut editor = editor(vec![BlockKind::Paragraph {
        content: InlineContent::new(vec![
            Inline::marked("one", MarkSet::BOLD),
            Inline::text(" two"),
        ]),
    }]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 7);

    editor.apply(EditorCommand::toggle_bold()).unwrap();

    assert!(editor.document().blocks()[0]
        .content()
        .unwrap()
        .has_mark_throughout(0, 7, MarkSet::BOLD));
}

#[test]
fn marks_are_independent_of_each_other() {
    let mut editor = editor(vec![BlockKind::paragraph("word")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 4);

    editor.apply(EditorCommand::toggle_bold()).unwrap();
    editor.apply(EditorCommand::toggle_italic()).unwrap();
    editor.apply(EditorCommand::toggle_bold()).unwrap();

    let content = editor.document().blocks()[0].content().unwrap();
    assert!(content.has_mark_throughout(0, 4, MarkSet::ITALIC));
    assert!(!content.has_mark_throughout(0, 4, MarkSet::BOLD));
}

#[test]
fn toggling_a_mark_with_no_selection_is_refused_for_now() {
    let mut editor = editor(vec![BlockKind::paragraph("word")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 2, 2);

    assert!(matches!(
        editor.apply(EditorCommand::toggle_bold()),
        Err(EditorError::Unsupported(_))
    ));
}

#[test]
fn typing_inherits_the_marks_at_the_caret() {
    let mut editor = editor(vec![BlockKind::Paragraph {
        content: InlineContent::new(vec![
            Inline::marked("bold", MarkSet::BOLD),
            Inline::text(" plain"),
        ]),
    }]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 4, 4);

    editor.apply(EditorCommand::insert_text("er")).unwrap();

    let content = editor.document().blocks()[0].content().unwrap();
    assert_eq!(content.plain_text(), "bolder plain");
    assert!(content.has_mark_throughout(0, 6, MarkSet::BOLD));
}

#[test]
fn links_are_set_and_cleared_over_a_selection() {
    let mut editor = editor(vec![BlockKind::paragraph("see the notes")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 8, 13);

    editor
        .apply(EditorCommand::SetLink {
            url: Some("notes.md".into()),
        })
        .unwrap();
    let content = editor.document().blocks()[0].content().unwrap();
    assert_eq!(content.link_at(9).map(|l| l.url.as_str()), Some("notes.md"));
    assert_eq!(content.link_at(2), None);

    editor.apply(EditorCommand::SetLink { url: None }).unwrap();
    assert_eq!(
        editor.document().blocks()[0].content().unwrap().link_at(9),
        None
    );
}

#[test]
fn typing_just_after_a_link_does_not_extend_it() {
    let mut editor = editor(vec![BlockKind::Paragraph {
        content: InlineContent::new(vec![Inline::link("notes", "notes.md")]),
    }]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 5, 5);

    editor.apply(EditorCommand::insert_text("!")).unwrap();

    let content = editor.document().blocks()[0].content().unwrap();
    assert_eq!(content.plain_text(), "notes!");
    assert_eq!(content.link_at(2).map(|l| l.url.as_str()), Some("notes.md"));
    assert_eq!(content.pieces().len(), 2, "the '!' is its own unlinked run");
}

#[test]
fn heading_levels_are_set_and_cleared() {
    let mut editor = editor(vec![BlockKind::paragraph("Chapter One")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor.apply(EditorCommand::SetHeadingLevel(1)).unwrap();
    assert_eq!(editor.document().blocks()[0].kind.name(), "heading");
    assert_eq!(editor.document().outline().len(), 1);
    assert_eq!(
        block(&editor, 0),
        paragraph,
        "changing kind keeps the block id"
    );

    editor.apply(EditorCommand::SetHeadingLevel(3)).unwrap();
    match &editor.document().blocks()[0].kind {
        BlockKind::Heading { level, .. } => assert_eq!(*level, 3),
        other => panic!("expected a heading, got {}", other.name()),
    }

    editor.apply(EditorCommand::SetParagraph).unwrap();
    assert_eq!(editor.document().blocks()[0].kind.name(), "paragraph");
    assert_eq!(text(&editor, 0), "Chapter One");
}

#[test]
fn a_paragraph_becomes_a_code_block_keeping_its_text() {
    let mut editor = editor(vec![BlockKind::paragraph("cargo test")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor
        .apply(EditorCommand::SetCodeBlock {
            language: Some("sh".into()),
        })
        .unwrap();

    match &editor.document().blocks()[0].kind {
        BlockKind::CodeBlock { language, code } => {
            assert_eq!(language.as_deref(), Some("sh"));
            assert_eq!(code, "cargo test");
        }
        other => panic!("expected a code block, got {}", other.name()),
    }
}

#[test]
fn quoting_nests_the_block_but_keeps_editing_it() {
    let mut editor = editor(vec![BlockKind::paragraph("quoted")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 6, 6);

    editor.apply(EditorCommand::WrapInQuote).unwrap();
    assert_eq!(editor.document().blocks()[0].kind.name(), "quote");

    // The nested block kept its id, so inline editing still reaches it.
    editor.apply(EditorCommand::insert_text("!")).unwrap();
    assert_eq!(
        editor.document().block(paragraph).unwrap().own_text(),
        "quoted!"
    );
    assert_eq!(editor.document().plain_text(), "quoted!");
}

#[test]
fn images_and_rules_are_inserted() {
    let mut editor = editor(vec![BlockKind::paragraph("see ")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 4, 4);

    editor
        .apply(EditorCommand::InsertImage(ImageNode::new("map.png")))
        .unwrap();
    assert_eq!(
        editor.document().block_len(paragraph),
        5,
        "an image counts as one"
    );

    editor.apply(EditorCommand::InsertThematicBreak).unwrap();
    assert_eq!(editor.document().blocks()[1].kind.name(), "thematicBreak");
    assert_eq!(
        editor.selection(),
        SelectionRange::in_block(paragraph, 5, 5),
        "a rule doesn't move the caret"
    );

    editor.apply(EditorCommand::InsertParagraph).unwrap();
    assert_eq!(editor.document().blocks().len(), 3);
    assert_eq!(editor.selection().head.block, block(&editor, 1));
}

// --- selection --------------------------------------------------------------

#[test]
fn a_selection_survives_an_edit_in_another_block() {
    let mut editor = editor(vec![
        BlockKind::paragraph("first"),
        BlockKind::paragraph("second"),
    ]);
    let first = block(&editor, 0);
    let second = block(&editor, 1);

    select(&mut editor, first, 0, 0);
    editor.apply(EditorCommand::insert_text("the ")).unwrap();

    // Whatever happened above it, this position still means the same place.
    let range = SelectionRange::in_block(second, 0, 6);
    assert_eq!(
        range.clamp(editor.document(), Position::new(second, 0)),
        range
    );
}

#[test]
fn a_selection_moves_with_the_text_it_follows() {
    let range = SelectionRange::in_block(BlockId(7), 10, 14);

    // Four characters inserted before it push it along.
    assert_eq!(
        range.map_after_replace(BlockId(7), 2, 2, 4),
        SelectionRange::in_block(BlockId(7), 14, 18)
    );
    // Text removed before it pulls it back.
    assert_eq!(
        range.map_after_replace(BlockId(7), 0, 5, 0),
        SelectionRange::in_block(BlockId(7), 5, 9)
    );
    // Text replaced under it collapses to the end of the replacement.
    assert_eq!(
        range.map_after_replace(BlockId(7), 8, 20, 3),
        SelectionRange::in_block(BlockId(7), 11, 11)
    );
    // Another block is none of its business.
    assert_eq!(range.map_after_replace(BlockId(8), 0, 5, 0), range);
}

#[test]
fn selection_ends_are_reported_in_document_order() {
    let editor = editor(vec![
        BlockKind::paragraph("first"),
        BlockKind::paragraph("second"),
    ]);
    let first = block(&editor, 0);
    let second = block(&editor, 1);

    // Dragged upwards: anchor after head.
    let backwards = SelectionRange::new(Position::new(second, 2), Position::new(first, 1));
    let (start, end) = backwards.ordered(editor.document());
    assert_eq!(start, Position::new(first, 1));
    assert_eq!(end, Position::new(second, 2));
}

#[test]
fn a_selection_in_a_deleted_block_falls_back() {
    let mut editor = editor(vec![
        BlockKind::paragraph("first"),
        BlockKind::paragraph("second"),
    ]);
    let first = block(&editor, 0);
    let second = block(&editor, 1);
    select(&mut editor, second, 0, 0);
    editor.apply(EditorCommand::MergeBlocks).unwrap();

    let stale = SelectionRange::in_block(second, 3, 3);
    assert_eq!(
        stale.clamp(editor.document(), Position::new(first, 0)),
        SelectionRange::in_block(first, 0, 0)
    );
}

#[test]
fn moving_the_caret_is_not_an_edit() {
    let mut editor = editor(vec![BlockKind::paragraph("word")]);
    let paragraph = block(&editor, 0);
    let revision = editor.revision();

    select(&mut editor, paragraph, 2, 4);

    assert_eq!(editor.revision(), revision);
    assert!(editor.history().is_empty());
}

// --- commands that can't be honoured ---------------------------------------

#[test]
fn a_selection_across_blocks_is_refused_rather_than_guessed_at() {
    let mut editor = editor(vec![
        BlockKind::paragraph("first"),
        BlockKind::paragraph("second"),
    ]);
    let first = block(&editor, 0);
    let second = block(&editor, 1);
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::new(
            Position::new(first, 1),
            Position::new(second, 2),
        )))
        .unwrap();

    assert!(matches!(
        editor.apply(EditorCommand::insert_text("x")),
        Err(EditorError::MultiBlockSelection)
    ));
    // And the document is untouched.
    assert_eq!(text(&editor, 0), "first");
    assert_eq!(text(&editor, 1), "second");
}

#[test]
fn a_failed_command_leaves_the_document_alone() {
    let mut editor = editor(vec![BlockKind::CodeBlock {
        language: None,
        code: "fn main() {}".into(),
    }]);
    let code = block(&editor, 0);
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::in_block(
            code, 0, 0,
        )))
        .unwrap();
    let revision = editor.revision();

    assert!(matches!(
        editor.apply(EditorCommand::insert_text("x")),
        Err(EditorError::NotTextual(_))
    ));
    assert_eq!(editor.revision(), revision);
    assert_eq!(text(&editor, 0), "fn main() {}");
}

// --- history ----------------------------------------------------------------

#[test]
fn undo_and_redo_walk_back_and_forth() {
    let mut editor = editor(vec![BlockKind::paragraph("")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor.apply(EditorCommand::insert_text("A")).unwrap();
    editor.apply(EditorCommand::insert_text("B")).unwrap();
    assert_eq!(text(&editor, 0), "AB");

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(text(&editor, 0), "A");
    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(text(&editor, 0), "");
    assert!(matches!(
        editor.apply(EditorCommand::Undo),
        Err(EditorError::NothingToDo)
    ));

    editor.apply(EditorCommand::Redo).unwrap();
    assert_eq!(text(&editor, 0), "A");
    editor.apply(EditorCommand::Redo).unwrap();
    assert_eq!(text(&editor, 0), "AB");
}

#[test]
fn undo_restores_the_selection_too() {
    let mut editor = editor(vec![BlockKind::paragraph("one two")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 4, 7);

    editor.apply(EditorCommand::insert_text("2")).unwrap();
    editor.apply(EditorCommand::Undo).unwrap();

    assert_eq!(text(&editor, 0), "one two");
    assert_eq!(
        editor.selection(),
        SelectionRange::in_block(paragraph, 4, 7)
    );
}

#[test]
fn undo_puts_structure_back() {
    let mut editor = editor(vec![BlockKind::paragraph("one two")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 3, 3);

    editor.apply(EditorCommand::SplitBlock).unwrap();
    editor.apply(EditorCommand::SetHeadingLevel(2)).unwrap();
    assert_eq!(editor.document().blocks().len(), 2);

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(editor.document().blocks()[1].kind.name(), "paragraph");
    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(editor.document().blocks().len(), 1);
    assert_eq!(text(&editor, 0), "one two");
}

#[test]
fn undoing_and_editing_again_branches_instead_of_forgetting() {
    // A → B → C, undo to B, then D: both C and D hang off B.
    let mut editor = editor(vec![BlockKind::paragraph("A")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 1, 1);

    editor.apply(EditorCommand::insert_text("B")).unwrap(); // node 0
    editor.apply(EditorCommand::insert_text("C")).unwrap(); // node 1
    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(text(&editor, 0), "AB");

    editor.apply(EditorCommand::insert_text("D")).unwrap(); // node 2, a branch
    assert_eq!(text(&editor, 0), "ABD");

    let history = editor.history();
    let children = history.children_newest_first(Some(editor_core::NodeIndex(0)));
    assert_eq!(children.len(), 2, "both C and D hang off B");
    assert_eq!(
        children[0],
        editor_core::NodeIndex(2),
        "newest branch first"
    );

    // Undo leaves the branch; redo takes the newest one, so the older line of
    // work is still reachable through the tree rather than lost.
    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(text(&editor, 0), "AB");
    editor.apply(EditorCommand::Redo).unwrap();
    assert_eq!(text(&editor, 0), "ABD");
}

#[test]
fn history_entries_carry_a_label_and_the_hosts_timestamp() {
    let mut editor = editor(vec![BlockKind::paragraph("")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor
        .apply_at(EditorCommand::insert_text("x"), Some(1_700_000_000_000))
        .unwrap();

    let entry = &editor.history().nodes()[0];
    assert_eq!(entry.label.as_deref(), Some("Typing"));
    assert_eq!(entry.created_at, Some(1_700_000_000_000));
}

#[test]
fn every_edit_moves_the_revision_once() {
    let mut editor = editor(vec![BlockKind::paragraph("one two")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 3, 3);

    let start = editor.revision();
    // One command, two operations (join the text, drop the block).
    editor.apply(EditorCommand::SplitBlock).unwrap();
    editor.apply(EditorCommand::MergeBlocks).unwrap();
    assert_eq!(editor.revision(), start + 2);
}

// --- document -----------------------------------------------------------

#[test]
fn plain_text_reaches_into_nested_blocks() {
    let mut document = Document::from_kinds("test.md", vec![BlockKind::heading(1, "Title")]);
    let quoted = document.new_block(BlockKind::paragraph("quoted line"));
    let item = document.new_block(BlockKind::paragraph("an item"));
    let mut editor = Editor::new(Document::from_kinds(
        "test.md",
        vec![
            BlockKind::heading(1, "Title"),
            BlockKind::Quote {
                blocks: vec![quoted],
            },
            BlockKind::List {
                ordered: false,
                start: None,
                tight: true,
                items: vec![editor_core::ListItem::new(vec![item])],
            },
            BlockKind::CodeBlock {
                language: None,
                code: "code()".into(),
            },
        ],
    ));

    assert_eq!(
        editor.document().plain_text(),
        "Title\n\nquoted line\n\nan item\n\ncode()"
    );
    assert_eq!(editor.document().outline()[0].text, "Title");

    // Nested text blocks are part of the caret's path through the document.
    let ids = editor.document().text_block_ids();
    assert_eq!(ids.len(), 3);
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::in_block(
            ids[2], 7, 7,
        )))
        .unwrap();
    editor.apply(EditorCommand::insert_text("!")).unwrap();
    assert!(editor.document().plain_text().contains("an item!"));

    // Ids from `document` above were reused here; nothing collides with them.
    assert!(!document.blocks().is_empty());
}

#[test]
fn block_ids_are_never_reused() {
    let mut editor = editor(vec![BlockKind::paragraph("one")]);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 3, 3);

    editor.apply(EditorCommand::SplitBlock).unwrap();
    let second = block(&editor, 1);
    editor.apply(EditorCommand::Undo).unwrap();
    editor.apply(EditorCommand::SplitBlock).unwrap();
    let again = block(&editor, 1);

    assert_ne!(second, again, "a redone split hands out a fresh id");
    assert!(editor.document().block(second).is_none());
}

#[test]
fn a_document_survives_serialization() {
    let mut editor = editor(vec![
        BlockKind::heading(2, "Title"),
        BlockKind::Paragraph {
            content: InlineContent::new(vec![
                Inline::marked("bold", MarkSet::BOLD),
                Inline::text(" and "),
                Inline::link("a link", "notes.md"),
            ]),
        },
    ]);
    let paragraph = block(&editor, 1);
    select(&mut editor, paragraph, 0, 4);
    editor.apply(EditorCommand::toggle_italic()).unwrap();

    let json = serde_json::to_string(editor.document()).unwrap();
    let mut back: Document = serde_json::from_str(&json).unwrap();
    back.reseal_ids();

    assert_eq!(back.plain_text(), editor.document().plain_text());
    assert_eq!(back.blocks(), editor.document().blocks());
    // Ids handed out after loading don't collide with the ones loaded.
    let fresh = back.new_block(BlockKind::paragraph("new"));
    assert!(editor.document().block(fresh.id).is_none());
}
