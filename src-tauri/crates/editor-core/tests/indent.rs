//! Indenting and outdenting list items: ⇥ and ⇧⇥.

use editor_core::{
    markdown, BlockId, Editor, EditorCommand, EditorError, Position, SelectionRange,
};

fn open(source: &str) -> Editor {
    Editor::new(markdown::parse("test.md", source))
}

/// The caret in the nth block a caret can go in, in document order.
fn caret_in(editor: &mut Editor, nth: usize) -> BlockId {
    let id = editor.document().text_block_ids()[nth];
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::collapsed(
            Position::new(id, 0),
        )))
        .unwrap();
    id
}

fn indent(editor: &mut Editor) -> Result<(), EditorError> {
    editor.apply(EditorCommand::IndentListItem).map(|_| ())
}

fn outdent(editor: &mut Editor) -> Result<(), EditorError> {
    editor.apply(EditorCommand::OutdentListItem).map(|_| ())
}

fn text(editor: &Editor) -> String {
    markdown::to_markdown(editor.document())
}

// --- indent -----------------------------------------------------------------

#[test]
fn indenting_makes_the_item_a_child_of_the_one_above() {
    let mut editor = open("- one\n- two");
    caret_in(&mut editor, 1);
    indent(&mut editor).unwrap();

    assert_eq!(text(&editor), "- one\n  - two\n");
}

#[test]
fn the_first_item_of_a_list_has_nothing_to_indent_into() {
    let mut editor = open("- one\n- two");
    caret_in(&mut editor, 0);

    assert_eq!(indent(&mut editor), Err(EditorError::NothingToDo));
    assert_eq!(text(&editor), "- one\n- two\n");
}

#[test]
fn indenting_twice_in_a_row_gives_one_nested_list() {
    let mut editor = open("- one\n- two\n- three");
    caret_in(&mut editor, 1);
    indent(&mut editor).unwrap();
    // `three` is now the second item of the outer list, so it indents into the
    // nested list that `two` is already in rather than making a second one.
    caret_in(&mut editor, 2);
    indent(&mut editor).unwrap();

    assert_eq!(text(&editor), "- one\n  - two\n  - three\n");
}

#[test]
fn the_only_item_of_a_nested_list_cannot_indent_further() {
    let mut editor = open("- one\n- two\n- three");
    caret_in(&mut editor, 2);
    indent(&mut editor).unwrap();
    assert_eq!(text(&editor), "- one\n- two\n  - three\n");

    // It is now the only item of the nested list, so there is nothing beside
    // it at that level to become a child of.
    caret_in(&mut editor, 2);
    assert_eq!(indent(&mut editor), Err(EditorError::NothingToDo));
}

#[test]
fn an_item_with_one_beside_it_goes_a_second_level_down() {
    let mut editor = open("- one\n  - two\n  - three");
    caret_in(&mut editor, 2);
    indent(&mut editor).unwrap();

    assert_eq!(text(&editor), "- one\n  - two\n    - three\n");
}

#[test]
fn indenting_keeps_the_caret_where_it_was() {
    let mut editor = open("- one\n- two");
    let id = caret_in(&mut editor, 1);
    indent(&mut editor).unwrap();

    // The block moved in the tree; its id, and so the caret, did not.
    assert_eq!(editor.selection().head.block, id);
    assert!(editor.document().block(id).is_some());
}

#[test]
fn indenting_carries_a_task_tick_with_it() {
    let mut editor = open("- [ ] one\n- [x] two");
    caret_in(&mut editor, 1);
    indent(&mut editor).unwrap();

    assert_eq!(text(&editor), "- [ ] one\n  - [x] two\n");
}

#[test]
fn indenting_carries_everything_nested_under_the_item() {
    let mut editor = open("- one\n- two\n  - under two");
    caret_in(&mut editor, 1);
    indent(&mut editor).unwrap();

    assert_eq!(text(&editor), "- one\n  - two\n    - under two\n");
}

#[test]
fn a_numbered_item_does_not_indent_into_a_bulleted_nest() {
    let mut editor = open("1. one\n   - bulleted\n2. two");
    caret_in(&mut editor, 2);
    indent(&mut editor).unwrap();

    // The bulleted list under `one` is a different list, not the numbered one
    // a level down, so `two` gets a numbered list of its own.
    assert_eq!(text(&editor), "1. one\n   - bulleted\n   1. two\n");
}

#[test]
fn indenting_outside_a_list_is_refused() {
    let mut editor = open("Just a paragraph.");
    caret_in(&mut editor, 0);

    assert!(matches!(
        indent(&mut editor),
        Err(EditorError::Unsupported(_))
    ));
}

// --- outdent ----------------------------------------------------------------

#[test]
fn outdenting_a_nested_item_joins_the_list_around_it() {
    let mut editor = open("- one\n  - two");
    caret_in(&mut editor, 1);
    outdent(&mut editor).unwrap();

    assert_eq!(text(&editor), "- one\n- two\n");
}

#[test]
fn outdenting_the_only_nested_item_takes_the_empty_list_with_it() {
    let mut editor = open("- one\n  - two\n- three");
    caret_in(&mut editor, 1);
    outdent(&mut editor).unwrap();

    // No empty nested list left behind under `one`.
    assert_eq!(text(&editor), "- one\n- two\n- three\n");
}

#[test]
fn outdenting_keeps_the_items_below_it_below_it() {
    let mut editor = open("- one\n  - two\n  - three\n  - four");
    caret_in(&mut editor, 1);
    outdent(&mut editor).unwrap();

    // `two` came out a level; `three` and `four` stay under it rather than
    // jumping out with it.
    assert_eq!(text(&editor), "- one\n- two\n  - three\n  - four\n");
}

#[test]
fn outdenting_a_top_level_item_stops_it_being_a_list_item() {
    let mut editor = open("- one\n- two");
    caret_in(&mut editor, 1);
    outdent(&mut editor).unwrap();

    assert_eq!(text(&editor), "- one\n\ntwo\n");
}

#[test]
fn outdenting_from_the_middle_of_a_top_level_list_splits_it() {
    let mut editor = open("- one\n- two\n- three\n- four\n- five");
    caret_in(&mut editor, 2);
    outdent(&mut editor).unwrap();

    assert_eq!(text(&editor), "- one\n- two\n\nthree\n\n- four\n- five\n");
}

#[test]
fn outdenting_the_only_item_of_a_top_level_list_leaves_no_list() {
    let mut editor = open("- one");
    caret_in(&mut editor, 0);
    outdent(&mut editor).unwrap();

    assert_eq!(text(&editor), "one\n");
}

#[test]
fn outdenting_the_first_item_of_a_top_level_list_leaves_no_empty_list_above() {
    let mut editor = open("- one\n- two");
    caret_in(&mut editor, 0);
    outdent(&mut editor).unwrap();

    assert_eq!(text(&editor), "one\n\n- two\n");
}

#[test]
fn outdenting_a_top_level_item_brings_its_nest_out_as_a_list() {
    let mut editor = open("- one\n  - under one");
    caret_in(&mut editor, 0);
    outdent(&mut editor).unwrap();

    // The level comes off the item, not off everything under it.
    assert_eq!(text(&editor), "one\n\n- under one\n");
}

#[test]
fn outdenting_keeps_the_caret_where_it_was() {
    let mut editor = open("- one\n  - two");
    let id = caret_in(&mut editor, 1);
    outdent(&mut editor).unwrap();

    assert_eq!(editor.selection().head.block, id);
    assert!(editor.document().block(id).is_some());
}

#[test]
fn outdenting_outside_a_list_is_refused() {
    let mut editor = open("Just a paragraph.");
    caret_in(&mut editor, 0);

    assert!(matches!(
        outdent(&mut editor),
        Err(EditorError::Unsupported(_))
    ));
}

// --- the two together -------------------------------------------------------

#[test]
fn indent_then_outdent_gives_back_what_was_there() {
    let source = "- one\n- two\n- three\n";
    let mut editor = open(source);
    caret_in(&mut editor, 1);
    indent(&mut editor).unwrap();
    caret_in(&mut editor, 1);
    outdent(&mut editor).unwrap();

    assert_eq!(text(&editor), source);
}

#[test]
fn indenting_undoes_in_one_step() {
    let mut editor = open("- one\n- two");
    caret_in(&mut editor, 1);
    indent(&mut editor).unwrap();
    editor.apply(EditorCommand::Undo).unwrap();

    assert_eq!(text(&editor), "- one\n- two\n");
}

#[test]
fn outdenting_undoes_in_one_step() {
    let mut editor = open("- one\n- two\n- three\n- four\n- five");
    caret_in(&mut editor, 2);
    outdent(&mut editor).unwrap();
    editor.apply(EditorCommand::Undo).unwrap();

    assert_eq!(text(&editor), "- one\n- two\n- three\n- four\n- five\n");
}
