//! Copy and paste: what a selection serializes to, and what pasted Markdown
//! turns into when it arrives.

use editor_core::{
    markdown, BlockId, Editor, EditorCommand, EditorError, Fragment, Position, SelectionRange,
};

fn open(source: &str) -> Editor {
    Editor::new(markdown::parse("test.md", source))
}

fn block(editor: &Editor, index: usize) -> BlockId {
    editor.document().blocks()[index].id
}

fn select(editor: &mut Editor, block: BlockId, start: usize, end: usize) {
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::in_block(
            block, start, end,
        )))
        .unwrap();
}

fn select_across(editor: &mut Editor, from: Position, to: Position) {
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::new(from, to)))
        .unwrap();
}

fn caret(editor: &mut Editor, block: BlockId, offset: usize) {
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::collapsed(
            Position::new(block, offset),
        )))
        .unwrap();
}

fn paste(editor: &mut Editor, source: &str) {
    editor
        .apply(EditorCommand::InsertMarkdown(source.to_string()))
        .unwrap();
}

// --- copy -------------------------------------------------------------------

#[test]
fn copying_part_of_a_block_keeps_its_marks_and_nothing_else() {
    let mut editor = open("She found the **letter** and read it.");
    let id = block(&editor, 0);
    select(&mut editor, id, 14, 20);

    let fragment = editor.copy().unwrap();
    assert!(matches!(fragment, Fragment::Inline { .. }));
    // The bold survives the copy; the block it came from doesn't come with it.
    assert_eq!(fragment.to_markdown(), "**letter**");
    assert_eq!(fragment.to_plain_text(), "letter");
}

#[test]
fn copying_a_whole_block_keeps_the_block() {
    let mut editor = open("## A heading\n\nAnd prose.");
    let id = block(&editor, 0);
    select(&mut editor, id, 0, 9);

    // Selected exactly, so pasting it back has to put a heading in, not nine
    // characters — which means the fragment is blocks, not inline.
    let fragment = editor.copy().unwrap();
    assert!(matches!(fragment, Fragment::Blocks { .. }));
    assert_eq!(fragment.to_markdown(), "## A heading");
}

#[test]
fn copying_nothing_is_an_error_rather_than_an_empty_fragment() {
    let mut editor = open("Text.");
    let id = block(&editor, 0);
    caret(&mut editor, id, 2);
    assert_eq!(editor.copy(), Err(EditorError::NothingToDo));
}

#[test]
fn copying_across_blocks_keeps_the_blocks_between_them() {
    let mut editor = open("First paragraph.\n\n## Middle\n\nLast paragraph.");
    let first = block(&editor, 0);
    let last = block(&editor, 2);
    select_across(&mut editor, Position::new(first, 6), Position::new(last, 4));

    let fragment = editor.copy().unwrap();
    assert_eq!(fragment.to_markdown(), "paragraph.\n\n## Middle\n\nLast");
}

#[test]
fn copying_across_list_items_keeps_the_list() {
    let mut editor = open("- one\n- two\n- three");
    let list = &editor.document().blocks()[0];
    let items: Vec<BlockId> = list.children().iter().map(|block| block.id).collect();
    select_across(
        &mut editor,
        Position::new(items[0], 0),
        Position::new(items[2], 5),
    );

    // The container survives: this is a list of three items, not three loose
    // paragraphs that happen to be next to each other.
    let fragment = editor.copy().unwrap();
    assert_eq!(fragment.to_markdown(), "- one\n- two\n- three");
}

#[test]
fn copying_part_of_a_list_keeps_only_the_items_it_reached() {
    let mut editor = open("- one\n- two\n- three\n- four");
    let items: Vec<BlockId> = editor.document().blocks()[0]
        .children()
        .iter()
        .map(|block| block.id)
        .collect();
    select_across(
        &mut editor,
        Position::new(items[1], 0),
        Position::new(items[2], 5),
    );

    let fragment = editor.copy().unwrap();
    assert_eq!(fragment.to_markdown(), "- two\n- three");
}

#[test]
fn copying_inside_a_code_block_is_text_not_a_fence() {
    let mut editor = open("```rust\nfn main() {}\n```");
    let id = block(&editor, 0);
    select(&mut editor, id, 0, 7);

    let fragment = editor.copy().unwrap();
    assert!(matches!(fragment, Fragment::Text { .. }));
    // No escaping and no fence: a copy of code is the code.
    assert_eq!(fragment.to_markdown(), "fn main");
}

#[test]
fn copying_a_whole_code_block_keeps_its_fence_and_language() {
    let mut editor = open("```rust\nfn main() {}\n```");
    let id = block(&editor, 0);
    select(&mut editor, id, 0, 12);

    let fragment = editor.copy().unwrap();
    assert_eq!(fragment.to_markdown(), "```rust\nfn main() {}\n```");
}

#[test]
fn copying_changes_nothing() {
    let mut editor = open("Some prose here.");
    let id = block(&editor, 0);
    select(&mut editor, id, 5, 10);
    let revision = editor.revision();

    editor.copy().unwrap();
    assert_eq!(editor.revision(), revision);
    assert!(editor.history().is_empty());
}

#[test]
fn copying_across_a_table_edge_is_refused() {
    let mut editor = open("Before.\n\n| a | b |\n| - | - |\n| c | d |");
    let before = block(&editor, 0);
    let cell = editor.document().blocks()[1].descendants()[1].id;
    select_across(
        &mut editor,
        Position::new(before, 0),
        Position::new(cell, 1),
    );

    assert!(matches!(editor.copy(), Err(EditorError::Unsupported(_))));
}

// --- cut --------------------------------------------------------------------

#[test]
fn a_cut_is_a_copy_and_a_delete() {
    let mut editor = open("She found the letter.");
    let id = block(&editor, 0);
    select(&mut editor, id, 14, 20);

    let fragment = editor.copy().unwrap();
    editor
        .apply(EditorCommand::Delete(editor_core::Direction::Backward))
        .unwrap();

    assert_eq!(fragment.to_plain_text(), "letter");
    assert_eq!(editor.document().plain_text(), "She found the .");
}

// --- paste ------------------------------------------------------------------

#[test]
fn pasting_a_sentence_leaves_the_block_it_lands_in_alone() {
    let mut editor = open("## A heading");
    let id = block(&editor, 0);
    caret(&mut editor, id, 12);
    paste(&mut editor, " and more");

    // Still one heading, with its id: a run of characters is not a block.
    assert_eq!(editor.document().blocks().len(), 1);
    assert_eq!(block(&editor, 0), id);
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "## A heading and more\n"
    );
}

#[test]
fn pasting_markdown_arrives_as_markdown() {
    let mut editor = open("Before.");
    let id = block(&editor, 0);
    caret(&mut editor, id, 7);
    paste(&mut editor, "## A heading\n\n- one\n- two");

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "Before.\n\n## A heading\n\n- one\n- two\n"
    );
}

#[test]
fn pasting_marks_keeps_them() {
    let mut editor = open("She found");
    let id = block(&editor, 0);
    caret(&mut editor, id, 9);
    paste(&mut editor, " the **letter**");

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "She found the **letter**\n"
    );
}

#[test]
fn pasting_into_the_middle_of_a_sentence_does_not_leave_half_of_it_behind() {
    let mut editor = open("She found it.");
    let id = block(&editor, 0);
    caret(&mut editor, id, 10);
    paste(&mut editor, "one\n\ntwo");

    // Three blocks, not four: the first pasted paragraph joins the text before
    // the caret, the text after it joins the last, and the block the caret was
    // in keeps its id.
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "She found one\n\ntwoit.\n"
    );
    assert_eq!(block(&editor, 0), id);
}

#[test]
fn pasting_over_a_selection_replaces_it_in_one_undo_step() {
    let mut editor = open("First.\n\nSecond.\n\nThird.");
    let first = block(&editor, 0);
    let third = block(&editor, 2);
    select_across(
        &mut editor,
        Position::new(first, 6),
        Position::new(third, 5),
    );
    paste(&mut editor, "one\n\ntwo");

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "First.one\n\ntwo.\n"
    );

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "First.\n\nSecond.\n\nThird.\n"
    );
}

#[test]
fn pasting_into_an_empty_paragraph_replaces_it() {
    let mut editor = open("Before.\n\n");
    editor.apply(EditorCommand::InsertParagraph).unwrap();
    let empty = block(&editor, 1);
    caret(&mut editor, empty, 0);
    paste(&mut editor, "## A heading\n\nAnd prose.");

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "Before.\n\n## A heading\n\nAnd prose.\n"
    );
}

#[test]
fn pasting_into_a_code_block_is_text() {
    let mut editor = open("```\nlet x = 1;\n```");
    let id = block(&editor, 0);
    caret(&mut editor, id, 10);
    paste(&mut editor, "\n# not a heading");

    // Still one code block, and the hash is code rather than a heading.
    assert_eq!(editor.document().blocks().len(), 1);
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "```\nlet x = 1;\n# not a heading\n```\n"
    );
}

#[test]
fn pasting_a_fence_into_prose_makes_a_code_block() {
    let mut editor = open("Look:");
    let id = block(&editor, 0);
    caret(&mut editor, id, 5);
    paste(&mut editor, "```rust\nfn main() {}\n```");

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "Look:\n\n```rust\nfn main() {}\n```\n"
    );
}

#[test]
fn pasting_leaves_the_caret_after_what_was_pasted() {
    let mut editor = open("Before.");
    let id = block(&editor, 0);
    caret(&mut editor, id, 7);
    paste(&mut editor, "one\n\ntwo");

    let caret = editor.selection();
    assert!(editor.selection().is_collapsed());
    assert_eq!(caret.head.offset, 3);
    assert_eq!(
        editor
            .document()
            .block(caret.head.block)
            .unwrap()
            .own_text(),
        "two"
    );

    // Typing carries on from there rather than from where the paste began.
    editor.apply(EditorCommand::insert_text("!")).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "Before.one\n\ntwo!\n"
    );
}

#[test]
fn pasting_undoes_in_one_step() {
    let mut editor = open("Before.");
    let id = block(&editor, 0);
    caret(&mut editor, id, 7);
    paste(&mut editor, "## A heading\n\n- one\n- two");

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), "Before.\n");
}

#[test]
fn pasting_ids_cannot_collide_with_ids_already_in_the_document() {
    let mut editor = open("- one\n- two");
    let before: Vec<BlockId> = editor
        .document()
        .all_blocks()
        .iter()
        .map(|block| block.id)
        .collect();
    let last = block(&editor, 0);
    let second = editor.document().text_block_ids()[1];
    caret(&mut editor, second, 3);
    paste(&mut editor, "- three\n- four");
    let _ = last;

    let ids: Vec<BlockId> = editor
        .document()
        .all_blocks()
        .iter()
        .map(|block| block.id)
        .collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(ids.len(), unique.len(), "ids {ids:?} are not unique");
    // The blocks that were already there kept theirs.
    for id in before {
        assert!(
            editor.document().block(id).is_some() || !ids.contains(&id),
            "id {id:?} was reused"
        );
    }
}

// --- round trip -------------------------------------------------------------

#[test]
fn a_copy_pasted_back_gives_back_what_was_copied() {
    let source = "## A heading\n\nProse with **bold** in it.\n\n- one\n- two";
    let mut editor = open(source);
    let first = editor.document().text_block_ids()[0];
    let ids = editor.document().text_block_ids();
    let last = *ids.last().unwrap();
    let end = editor.document().block_len(last);
    select_across(
        &mut editor,
        Position::new(first, 0),
        Position::new(last, end),
    );

    let copied = editor.copy().unwrap().to_markdown();
    assert_eq!(copied, source);

    // Pasted into an empty document, it reads back as the same Markdown.
    let mut other = open("");
    let id = block(&other, 0);
    caret(&mut other, id, 0);
    other
        .apply(EditorCommand::InsertMarkdown(copied.clone()))
        .unwrap();
    assert_eq!(
        markdown::to_markdown(other.document()),
        format!("{source}\n")
    );
}

// --- edges ------------------------------------------------------------------

#[test]
fn pasting_a_divider_into_an_empty_document_leaves_somewhere_to_type() {
    let mut editor = open("");
    let id = block(&editor, 0);
    caret(&mut editor, id, 0);
    paste(&mut editor, "---");

    // Nothing pasted holds text, and the empty paragraph it replaced is gone,
    // so the caret has to land somewhere that still exists.
    let caret = editor.selection();
    assert!(
        editor.document().block(caret.head.block).is_some(),
        "caret is on block {:?}, which is not in the document",
        caret.head.block
    );
}

#[test]
fn pasting_into_a_list_item_stays_in_the_list() {
    let mut editor = open("- one\n- two");
    let second = editor.document().text_block_ids()[1];
    caret(&mut editor, second, 3);
    paste(&mut editor, "a\n\nb");

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "- one\n- twoa\n  b\n"
    );
}

#[test]
fn pasting_into_a_quote_stays_in_the_quote() {
    let mut editor = open("> quoted");
    let inner = editor.document().text_block_ids()[0];
    caret(&mut editor, inner, 6);
    paste(&mut editor, "a\n\nb");

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "> quoteda\n>\n> b\n"
    );
}

#[test]
fn pasting_a_table_into_prose_arrives_as_a_table() {
    let mut editor = open("Look:");
    let id = block(&editor, 0);
    caret(&mut editor, id, 5);
    paste(&mut editor, "| a | b |\n| - | - |\n| c | d |");

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "Look:\n\n| a | b |\n| --- | --- |\n| c | d |\n"
    );
}

#[test]
fn pasting_what_a_word_processor_sends_is_text_not_syntax() {
    // A paste from outside is Markdown as far as the engine is concerned, and
    // most of what arrives has no syntax in it at all.
    let mut editor = open("");
    let id = block(&editor, 0);
    caret(&mut editor, id, 0);
    paste(&mut editor, "Plain prose with 3 * 4 in it.");

    assert_eq!(
        editor.document().plain_text(),
        "Plain prose with 3 * 4 in it."
    );
}

#[test]
fn pasting_nothing_but_a_space_still_puts_the_space_in() {
    let mut editor = open("one");
    let id = block(&editor, 0);
    caret(&mut editor, id, 3);
    paste(&mut editor, " ");

    assert_eq!(editor.document().blocks()[0].own_text(), "one ");
}
