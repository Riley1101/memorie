//! Where the time goes on one keystroke.
//!
//! Not a benchmark harness — a test that prints numbers, so
//! `cargo test --release --test perf -- --nocapture` answers "is the engine the
//! problem?" without adding a dependency. The shape of a real document matters
//! more than the precision here: 800 blocks, ~116k characters, matching what
//! the spike measured in the app.

use editor_core::{markdown, BlockKind, Document, Editor, EditorCommand, SelectionRange};
use std::time::Instant;

fn big_document() -> String {
    let paragraph = "She turned the letter over twice before opening it, and the hallway light \
                     caught the seal in a way that made the wax look almost wet again. ";
    (0..400)
        .map(|index| format!("## Section {index}\n\n{}", paragraph.repeat(2)))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn time<T>(label: &str, iterations: usize, mut work: impl FnMut() -> T) -> f64 {
    // One warm-up, then the median of `iterations` runs.
    work();
    let mut times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        let value = work();
        times.push(started.elapsed().as_secs_f64() * 1000.0);
        drop(value);
    }
    times.sort_by(f64::total_cmp);
    let median = times[times.len() / 2];
    println!("{label:<34} {median:>8.3}ms");
    median
}

#[test]
fn where_one_keystroke_goes() {
    let source = big_document();
    println!(
        "\ndocument: {} chars, {} blocks\n",
        source.len(),
        markdown::parse("big.md", &source).blocks().len()
    );

    let document = markdown::parse("big.md", &source);
    let mut editor = Editor::new(document);
    let target = editor.document().text_block_ids()[400];
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::in_block(
            target, 5, 5,
        )))
        .unwrap();

    time("parse markdown", 5, || markdown::parse("big.md", &source));
    time("serialize markdown", 5, || {
        markdown::to_markdown(editor.document())
    });
    time("insert one character", 200, || {
        editor.apply(EditorCommand::insert_text("x")).unwrap()
    });
    time("document.block(id)", 200, || {
        editor.document().block(target).map(|b| b.id)
    });
    time("text_block_ids", 200, || {
        editor.document().text_block_ids().len()
    });
    time("plain_text", 20, || editor.document().plain_text().len());
    time("outline", 200, || editor.document().outline().len());
    time("serde_json of the document", 20, || {
        serde_json::to_string(editor.document()).map(|json| json.len())
    });
}

#[test]
fn a_small_document_is_not_the_problem() {
    let mut editor = Editor::new(Document::from_kinds(
        "small.md",
        vec![BlockKind::paragraph("one two three")],
    ));
    let block = editor.document().blocks()[0].id;
    editor
        .apply(EditorCommand::SetSelection(SelectionRange::in_block(
            block, 3, 3,
        )))
        .unwrap();
    println!();
    time("insert into a 1-block document", 500, || {
        editor.apply(EditorCommand::insert_text("x")).unwrap()
    });
}
