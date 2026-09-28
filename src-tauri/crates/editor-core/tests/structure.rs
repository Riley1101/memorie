//! Selections that span blocks, list and table commands, and typing that
//! collapses into one undo step.

use editor_core::{
    markdown, BlockId, BlockKind, Direction, Document, Editor, EditorCommand, EditorError,
    Position, SelectionRange,
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

// --- selections across blocks -----------------------------------------------

#[test]
fn deleting_across_blocks_joins_what_is_left() {
    let mut editor = open("one two\n\nthree four\n");
    let first = block(&editor, 0);
    let second = block(&editor, 1);
    select_across(
        &mut editor,
        Position::new(first, 4),
        Position::new(second, 6),
    );

    editor
        .apply(EditorCommand::Delete(Direction::Backward))
        .unwrap();

    assert_eq!(editor.document().blocks().len(), 1);
    assert_eq!(editor.document().plain_text(), "one four");
    assert_eq!(editor.selection(), SelectionRange::in_block(first, 4, 4));
}

#[test]
fn typing_over_a_selection_across_blocks_replaces_all_of_it() {
    let mut editor = open("one two\n\nmiddle\n\nthree four\n");
    let first = block(&editor, 0);
    let last = block(&editor, 2);
    select_across(&mut editor, Position::new(first, 4), Position::new(last, 6));

    editor.apply(EditorCommand::insert_text("and ")).unwrap();

    assert_eq!(
        editor.document().blocks().len(),
        1,
        "the blocks between go too"
    );
    assert_eq!(editor.document().plain_text(), "one and four");
    assert_eq!(editor.selection(), SelectionRange::in_block(first, 8, 8));
}

#[test]
fn a_backwards_selection_across_blocks_deletes_the_same_text() {
    let mut editor = open("one two\n\nthree four\n");
    let first = block(&editor, 0);
    let second = block(&editor, 1);
    // Dragged upwards: the head is before the anchor.
    select_across(
        &mut editor,
        Position::new(second, 6),
        Position::new(first, 4),
    );

    editor
        .apply(EditorCommand::Delete(Direction::Forward))
        .unwrap();

    assert_eq!(editor.document().plain_text(), "one four");
}

#[test]
fn undoing_a_cross_block_delete_puts_the_blocks_back() {
    let source = "one two\n\nmiddle\n\nthree four\n";
    let mut editor = open(source);
    let first = block(&editor, 0);
    let last = block(&editor, 2);
    select_across(&mut editor, Position::new(first, 4), Position::new(last, 6));

    editor
        .apply(EditorCommand::Delete(Direction::Backward))
        .unwrap();
    editor.apply(EditorCommand::Undo).unwrap();

    assert_eq!(markdown::to_markdown(editor.document()), source);
    assert_eq!(
        editor.selection(),
        SelectionRange::new(Position::new(first, 4), Position::new(last, 6)),
        "and the selection with them"
    );
}

#[test]
fn a_cross_block_delete_keeps_marks_on_both_sides() {
    let mut editor = open("**bold text**\n\n_italic text_\n");
    let first = block(&editor, 0);
    let second = block(&editor, 1);
    select_across(
        &mut editor,
        Position::new(first, 4),
        Position::new(second, 7),
    );

    editor
        .apply(EditorCommand::Delete(Direction::Backward))
        .unwrap();

    assert_eq!(markdown::to_markdown(editor.document()), "**bold**_text_\n");
}

// --- lists ------------------------------------------------------------------

#[test]
fn a_paragraph_becomes_a_list_and_back() {
    let mut editor = open("an item\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 2, 2);

    editor
        .apply(EditorCommand::ToggleList { ordered: false })
        .unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), "- an item\n");
    // The paragraph kept its id, so the caret is still in it and typing works.
    editor.apply(EditorCommand::insert_text("other")).unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), "- another item\n");

    editor
        .apply(EditorCommand::ToggleList { ordered: false })
        .unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), "another item\n");
}

#[test]
fn a_list_flips_between_bullets_and_numbers() {
    let mut editor = open("- one\n- two\n");
    let item = editor.document().text_block_ids()[0];
    select(&mut editor, item, 0, 0);

    editor
        .apply(EditorCommand::ToggleList { ordered: true })
        .unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), "1. one\n2. two\n");

    editor
        .apply(EditorCommand::ToggleList { ordered: false })
        .unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), "- one\n- two\n");
}

#[test]
fn a_nested_list_flips_without_disturbing_the_one_around_it() {
    let mut editor = open("- one\n  - inner\n- two\n");
    let inner = editor.document().text_block_ids()[1];
    select(&mut editor, inner, 0, 0);

    editor
        .apply(EditorCommand::ToggleList { ordered: true })
        .unwrap();

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "- one\n  1. inner\n- two\n"
    );
}

#[test]
fn unwrapping_a_nested_list_is_refused_rather_than_guessed_at() {
    let mut editor = open("- one\n  - inner\n");
    let inner = editor.document().text_block_ids()[1];
    select(&mut editor, inner, 0, 0);

    assert!(matches!(
        editor.apply(EditorCommand::ToggleList { ordered: false }),
        Err(EditorError::NestedBlock(_))
    ));
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "- one\n  - inner\n"
    );
}

#[test]
fn a_list_item_becomes_a_task_then_ticks_and_unticks() {
    let mut editor = open("- one\n- two\n");
    let second = editor.document().text_block_ids()[1];
    select(&mut editor, second, 0, 0);

    editor.apply(EditorCommand::ToggleTask).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "- one\n- [ ] two\n"
    );

    editor.apply(EditorCommand::ToggleTask).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "- one\n- [x] two\n"
    );

    editor.apply(EditorCommand::ToggleTask).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "- one\n- [ ] two\n"
    );

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "- one\n- [x] two\n"
    );
}

#[test]
fn toggling_a_task_outside_a_list_says_so() {
    let mut editor = open("prose\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    assert!(matches!(
        editor.apply(EditorCommand::ToggleTask),
        Err(EditorError::Unsupported(_))
    ));
}

// --- tables -----------------------------------------------------------------

#[test]
fn a_table_is_inserted_with_the_caret_in_its_first_cell() {
    let mut editor = open("before\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 6, 6);

    editor
        .apply(EditorCommand::InsertTable {
            rows: 1,
            columns: 2,
        })
        .unwrap();

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "before\n\n|  |  |\n| --- | --- |\n|  |  |\n"
    );
    editor.apply(EditorCommand::insert_text("Name")).unwrap();
    assert!(markdown::to_markdown(editor.document()).contains("| Name |  |"));
}

#[test]
fn rows_and_columns_are_added_around_the_caret() {
    let mut editor = open("| a | b |\n| --- | --- |\n| c | d |\n");
    let cell = editor.document().text_block_ids()[2]; // the cell holding "c"
    select(&mut editor, cell, 0, 0);

    editor
        .apply(EditorCommand::InsertTableRow { before: false })
        .unwrap();
    editor.apply(EditorCommand::insert_text("e")).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "| a | b |\n| --- | --- |\n| c | d |\n| e |  |\n"
    );

    select(&mut editor, cell, 1, 1);
    editor
        .apply(EditorCommand::InsertTableColumn { before: false })
        .unwrap();
    editor.apply(EditorCommand::insert_text("x")).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "| a |  | b |\n| --- | --- | --- |\n| c | x | d |\n| e |  |  |\n"
    );
}

#[test]
fn a_row_inserted_before_the_header_lands_under_it() {
    let mut editor = open("| a |\n| --- |\n| b |\n");
    let header = editor.document().text_block_ids()[0];
    select(&mut editor, header, 0, 0);

    editor
        .apply(EditorCommand::InsertTableRow { before: true })
        .unwrap();
    editor.apply(EditorCommand::insert_text("new")).unwrap();

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "| a |\n| --- |\n| new |\n| b |\n"
    );
}

#[test]
fn editing_a_cell_leaves_the_rest_of_the_table_alone() {
    let source = "| a | b |\n| :--- | ---: |\n| c | d |\n";
    let mut editor = open(source);
    let cell = editor.document().text_block_ids()[3]; // "d"
    select(&mut editor, cell, 1, 1);

    editor.apply(EditorCommand::insert_text("!")).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "| a | b |\n| :--- | ---: |\n| c | d! |\n"
    );

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), source);
}

#[test]
fn a_table_command_outside_a_table_says_so() {
    let mut editor = open("prose\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    assert!(matches!(
        editor.apply(EditorCommand::InsertTableRow { before: false }),
        Err(EditorError::Unsupported(_))
    ));
}

// --- coalescing -------------------------------------------------------------

#[test]
fn a_burst_of_typing_is_one_undo_step() {
    let mut editor = Editor::new(Document::from_kinds(
        "test.md",
        vec![BlockKind::paragraph("")],
    ));
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    let mut at = 1_700_000_000_000;
    for letter in "hello".chars() {
        editor
            .apply_at(EditorCommand::insert_text(letter.to_string()), Some(at))
            .unwrap();
        at += 100;
    }
    assert_eq!(editor.document().plain_text(), "hello");
    assert_eq!(
        editor.history().nodes().len(),
        1,
        "one history entry for the burst"
    );

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(editor.document().plain_text(), "");
    editor.apply(EditorCommand::Redo).unwrap();
    assert_eq!(editor.document().plain_text(), "hello");
    assert_eq!(
        editor.selection(),
        SelectionRange::in_block(paragraph, 5, 5)
    );
}

#[test]
fn a_pause_starts_a_new_undo_step() {
    let mut editor = Editor::new(Document::from_kinds(
        "test.md",
        vec![BlockKind::paragraph("")],
    ));
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor
        .apply_at(EditorCommand::insert_text("one"), Some(1_000))
        .unwrap();
    editor
        .apply_at(EditorCommand::insert_text(" two"), Some(1_000 + 5_000))
        .unwrap();

    assert_eq!(editor.history().nodes().len(), 2);
    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(editor.document().plain_text(), "one");
}

#[test]
fn moving_the_caret_breaks_the_burst() {
    let mut editor = Editor::new(Document::from_kinds(
        "test.md",
        vec![BlockKind::paragraph("word")],
    ));
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 4, 4);

    editor
        .apply_at(EditorCommand::insert_text("s"), Some(1_000))
        .unwrap();
    select(&mut editor, paragraph, 0, 0);
    editor
        .apply_at(EditorCommand::insert_text("A "), Some(1_100))
        .unwrap();

    assert_eq!(editor.document().plain_text(), "A words");
    assert_eq!(editor.history().nodes().len(), 2, "two places, two steps");
    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(editor.document().plain_text(), "words");
}

#[test]
fn only_typing_coalesces() {
    let mut editor = Editor::new(Document::from_kinds(
        "test.md",
        vec![BlockKind::paragraph("one")],
    ));
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 3, 3);

    editor
        .apply_at(EditorCommand::SplitBlock, Some(1_000))
        .unwrap();
    editor
        .apply_at(EditorCommand::SplitBlock, Some(1_010))
        .unwrap();

    assert_eq!(editor.history().nodes().len(), 2);
}

#[test]
fn without_timestamps_nothing_coalesces() {
    let mut editor = Editor::new(Document::from_kinds(
        "test.md",
        vec![BlockKind::paragraph("")],
    ));
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor.apply(EditorCommand::insert_text("a")).unwrap();
    editor.apply(EditorCommand::insert_text("b")).unwrap();

    // Guessing that two undated edits belong together would be worse than an
    // extra undo press.
    assert_eq!(editor.history().nodes().len(), 2);
}

#[test]
fn a_coalesced_burst_does_not_swallow_a_branch() {
    let mut editor = Editor::new(Document::from_kinds(
        "test.md",
        vec![BlockKind::paragraph("")],
    ));
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor
        .apply_at(EditorCommand::insert_text("a"), Some(1_000))
        .unwrap();
    editor
        .apply_at(EditorCommand::insert_text("b"), Some(1_050))
        .unwrap();
    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(editor.document().plain_text(), "");

    // The undone burst is a node with history of its own; typing again must
    // branch off its parent rather than reopen it.
    editor
        .apply_at(EditorCommand::insert_text("c"), Some(1_100))
        .unwrap();
    assert_eq!(editor.document().plain_text(), "c");
    assert_eq!(editor.history().nodes().len(), 2);
}
