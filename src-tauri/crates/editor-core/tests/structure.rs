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

// --- text reported back by the view (composition, autocorrect, paste) -------

#[test]
fn reported_text_becomes_the_smallest_edit_that_explains_it() {
    // What an IME leaves behind: the block now reads differently, and only the
    // before/after pair says what happened.
    let mut editor = open("I typed nihongo here\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 20, 20);

    editor
        .apply(EditorCommand::SetBlockText("I typed 日本語 here".into()))
        .unwrap();

    assert_eq!(editor.document().plain_text(), "I typed 日本語 here");
    // The caret sits after the composed text, not at the end of the block.
    assert_eq!(
        editor.selection(),
        SelectionRange::in_block(paragraph, 11, 11)
    );
}

#[test]
fn reported_text_leaves_formatting_on_either_side_alone() {
    let mut editor = open("**bold** plain _italic_\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    // Only the middle word changed.
    editor
        .apply(EditorCommand::SetBlockText("bold PLAIN italic".into()))
        .unwrap();

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "**bold** PLAIN _italic_\n"
    );
}

#[test]
fn text_composed_inside_a_marked_run_keeps_its_marks() {
    let mut editor = open("**bold**\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 4, 4);

    editor
        .apply(EditorCommand::SetBlockText("bolder".into()))
        .unwrap();

    assert_eq!(markdown::to_markdown(editor.document()), "**bolder**\n");
}

#[test]
fn reported_text_can_delete_and_can_be_undone_in_one_step() {
    let source = "one two three\n";
    let mut editor = open(source);
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor
        .apply(EditorCommand::SetBlockText("one three".into()))
        .unwrap();
    assert_eq!(editor.document().plain_text(), "one three");

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), source);
}

#[test]
fn reported_text_counts_an_atom_as_one_character() {
    // The view writes U+FFFC where an image sits, so offsets line up.
    let mut editor = open("see ![a map](map.png) here\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);

    editor
        .apply(EditorCommand::SetBlockText("see \u{FFFC} there".into()))
        .unwrap();

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "see ![a map](map.png) there\n"
    );
}

#[test]
fn reporting_unchanged_text_does_nothing() {
    let mut editor = open("unchanged\n");
    let paragraph = block(&editor, 0);
    select(&mut editor, paragraph, 0, 0);
    let revision = editor.revision();

    assert!(matches!(
        editor.apply(EditorCommand::SetBlockText("unchanged".into())),
        Err(EditorError::NothingToDo)
    ));
    assert_eq!(editor.revision(), revision);
}

// --- code blocks ------------------------------------------------------------

#[test]
fn a_caret_can_type_in_a_code_block() {
    let mut editor = open("```rust\nfn main() {}\n```\n");
    let code = editor.document().text_block_ids()[0];
    select(&mut editor, code, 12, 12);

    editor.apply(EditorCommand::insert_text(" // hi")).unwrap();

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "```rust\nfn main() {} // hi\n```\n"
    );
    assert_eq!(editor.selection(), SelectionRange::in_block(code, 18, 18));
}

#[test]
fn return_inside_code_adds_a_line_rather_than_a_second_fence() {
    let mut editor = open("```\none\n```\n");
    let code = editor.document().text_block_ids()[0];
    select(&mut editor, code, 3, 3);

    editor.apply(EditorCommand::SplitBlock).unwrap();
    editor.apply(EditorCommand::insert_text("two")).unwrap();

    assert_eq!(editor.document().blocks().len(), 1, "still one code block");
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "```\none\ntwo\n```\n"
    );
}

#[test]
fn deleting_in_code_stops_at_its_edges() {
    let mut editor = open("before\n\n```\nxy\n```\n");
    let code = editor.document().text_block_ids()[1];

    select(&mut editor, code, 1, 1);
    editor
        .apply(EditorCommand::Delete(Direction::Backward))
        .unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "before\n\n```\ny\n```\n"
    );

    // At the start there is nothing to join to: merging code into prose would
    // lose the fence, so the delete is refused rather than guessed at.
    select(&mut editor, code, 0, 0);
    assert!(matches!(
        editor.apply(EditorCommand::Delete(Direction::Backward)),
        Err(EditorError::NothingToDo)
    ));
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "before\n\n```\ny\n```\n"
    );
}

#[test]
fn code_edits_undo_and_keep_the_language() {
    let source = "```python\nprint(1)\n```\n";
    let mut editor = open(source);
    let code = editor.document().text_block_ids()[0];
    select(&mut editor, code, 6, 7);

    editor.apply(EditorCommand::insert_text("42")).unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "```python\nprint(42)\n```\n"
    );

    editor.apply(EditorCommand::Undo).unwrap();
    assert_eq!(markdown::to_markdown(editor.document()), source);
}

#[test]
fn code_reported_back_by_the_view_becomes_the_smallest_edit() {
    let mut editor = open("```\nlet a = 1;\n```\n");
    let code = editor.document().text_block_ids()[0];
    select(&mut editor, code, 0, 0);

    editor
        .apply(EditorCommand::SetBlockText("let alpha = 1;".into()))
        .unwrap();

    assert_eq!(
        markdown::to_markdown(editor.document()),
        "```\nlet alpha = 1;\n```\n"
    );
}

#[test]
fn offsets_in_code_count_characters() {
    let mut editor = open("```\ncafé 日本語\n```\n");
    let code = editor.document().text_block_ids()[0];
    assert_eq!(editor.document().block_len(code), 8);

    select(&mut editor, code, 5, 8);
    editor
        .apply(EditorCommand::insert_text("にほんご"))
        .unwrap();
    assert_eq!(
        markdown::to_markdown(editor.document()),
        "```\ncafé にほんご\n```\n"
    );
}

#[test]
fn a_backspace_at_a_code_blocks_seam_does_nothing_rather_than_failing() {
    // Joining prose and code would lose the fence, so the delete is refused —
    // and refused cleanly, before any operation has been applied.
    let source = "prose\n\n```\ncode\n```\n\nmore\n";
    let mut editor = open(source);
    let after = editor.document().text_block_ids()[2];
    select(&mut editor, after, 0, 0);

    assert!(matches!(
        editor.apply(EditorCommand::Delete(Direction::Backward)),
        Err(EditorError::NothingToDo)
    ));
    assert_eq!(markdown::to_markdown(editor.document()), source);
}

#[test]
fn words_are_counted_without_building_the_document() {
    let editor = open("# Title\n\n> quoted words here\n\n- one two\n\n```\ncode here\n```\n");
    // Title, the quote, the list item and the code: nested blocks included.
    assert_eq!(editor.document().word_count(), 8);
    assert_eq!(
        editor.document().word_count(),
        editor.document().plain_text().split_whitespace().count(),
        "the cheap count agrees with the one that builds the text"
    );
}
