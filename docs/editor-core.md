# The Rust editor core

This document records where the document engine lives today (Phase 1), the
model the Rust engine uses and why (Phase 2), and what each later phase has to
do. It is the place to argue with the design before more code depends on it.

## 1. Where document state lives today

The editor is **Milkdown 7** (ProseMirror + remark) in
[editor.svelte](../src/lib/components/editor.svelte). There is already a real
document model in the app — it is just in JavaScript, inside ProseMirror.

The flow for one keystroke:

```text
keypress
  → ProseMirror transaction (the real document model, in JS)
  → listenerCtx.updated
  → debounced 2s (5s over 100k positions)
  → getMarkdown()               serializes the whole document
  → md-editor.onSave
  → serializeWriting(meta, extra, markdown)   front matter put back, in JS
  → invoke('update_file')
  → fs::update_file + undotree.add_change(name, whole text)
  → file watcher → indexer pass → chunker → embeddings → SurrealDB
```

Facts that matter for the migration:

- **Rust never sees a document, only file text.** Every Rust consumer
  (`chunker`, `indexer`, `search`, `export`, `pdf`, `codex`) re-parses Markdown
  from scratch, three of them with their own `pulldown_cmark` setup.
- **Undo/redo is two unrelated systems.** ProseMirror's in-editor history (keystroke
  granularity, in JS, lost on reload) and
  [`undotree.rs`](../src-tauri/src/undotree.rs) (per *save*, on disk, branching,
  delta-compressed with keyframes every 32 versions). `undo_file` /
  `redo_file` / `goto_file_version` return **whole document text**, which the
  frontend then remounts the editor with (`appState.ui.editorVersion`).
- **`read_file` prefers the undo tree over the file on disk.** Version history
  is the read path, not just an archive.
- **Front matter is parsed and written in JavaScript**
  ([front-matter.js](../src/lib/front-matter.js)), byte-preserving when metadata
  is unchanged. Rust's `chunker` separately knows how to skip YAML metadata.
  Two implementations of the same rule.
- **Outline, word count and selection text are recomputed by walking the whole
  ProseMirror document** on a 250ms/1s debounce — the reason the large-document
  thresholds in `editor.svelte` exist.
- **Editing commands are Milkdown's**: `toggleStrongCommand`,
  `wrapInHeadingCommand` and friends, driven from
  [slash-menu.svelte.js](../src/lib/components/plugins/slash-menu.svelte.js) and
  the command bar.

So there is no missing document model to build from nothing; there are **two
half-models** (ProseMirror's, and "text plus whatever each Rust module
re-parses") with Markdown strings as the only thing they share.

## 2. What moves to Rust

- The document model, selection model, editing commands, transactions and
  in-session history.
- Markdown parsing and serialization as **one** implementation (Phase 3),
  replacing the per-module `pulldown_cmark` walks where they duplicate it.
- Front matter, eventually: one parser, in Rust, keeping the current
  byte-preserving behaviour.
- Structural queries the UI asks for on a timer today: outline, word count,
  plain text, headings.

## 3. What stays in Svelte

Everything the user sees: layout, binder/tree, sidebar, settings, dialogs,
command palette, menus, outline, search UI, AI panels, status bar, find bar,
slash menu, link popover, codex highlights, focus mode, typewriter scrolling.

Also, for now, **all DOM-level editing mechanics**: caret painting, IME,
composition, spellcheck, drag and drop. The engine owns document state; the
view owns how a caret feels. Phase 5 decides how much of Milkdown remains as a
view layer — see the risks.

## 4. Crate layout

```text
src-tauri/
  Cargo.toml                 workspace root (member: crates/editor-core)
  crates/editor-core/        no tauri, no tokio, no filesystem; serde only
    src/ids.rs               BlockId, DocumentId, id generator
    src/inline.rs            MarkSet, Inline, InlineContent (flat runs)
    src/markdown.rs          parse / to_markdown, front matter, DocumentSource
    src/node.rs              Block, BlockKind, ListItem, TableNode
    src/document.rs          Document, metadata, plain_text, outline
    src/selection.rs         Position, SelectionRange, mapping
    src/command.rs           EditorCommand
    src/transaction.rs       Operation, Transaction (atomic, invertible)
    src/history.rs           branching history of transactions
    src/editor.rs            Editor: command → transaction → document + history
    src/error.rs             EditorError
    tests/editing.rs         editing, selection and history behaviour
    tests/markdown.rs        parsing, printing, round-trip properties
    tests/fixtures/*.md      real documents, in the engine's own spelling
```

A separate crate rather than a module in `memoire`, because "reusable" has to
be enforced by the compiler: `editor-core` cannot reach for Tauri state, the
config, or the filesystem even by accident. It sits under `src-tauri/` so the
Tauri build, the existing `target/` directory and CI are untouched; moving it
to the repo root later is a directory move and one path in `Cargo.toml`.

`src-tauri/Cargo.toml` became the workspace root. Nothing else about the
`memoire` crate changed in Phase 2 — it does not yet depend on `editor-core`.

## 5. Model decisions worth reviewing

**Markdown is not the state.** `Document` is. Nothing in `editor-core` parses
or prints Markdown yet, and when it does (Phase 3) it will be one module with
`Document` on both sides.

**Blocks carry ids; positions never carry indices.** `Position` is
`{ block: BlockId, offset }`. An edit in block 2 leaves a position in block 9
alone — no mapping, no drift. This is what lets grammar results, codex
mentions, comments and (later) collaborative cursors survive edits. Ids are
monotonic and never reused, so a stale id resolves to nothing rather than to
the wrong block.

**Inline content is flat, not a tree.** `**bold _and italic_**` is a spelling,
not a structure: what is being edited is a run of characters carrying a set of
marks. So a block's content is a normalised `Vec<Inline>` of text runs with
`MarkSet` plus optional link, and atoms (image, hard break). Consequences:
inserting touches at most two runs; toggling bold is a mark change, not a tree
rebuild; "is this bold?" is a scan; and two spellings of the same text
normalise equal, which is what makes round-tripping stable. The serializer
rebuilds nesting on the way out.

**Offsets count `char`s.** Not bytes (they land inside characters), not UTF-16
(that is a JavaScript detail the engine should not carry). The Tauri layer
converts at the boundary, where the frontend's units are known. This is a real
piece of work in Phase 5 and the most likely source of off-by-one bugs.

**Two operations, not twelve.** `ReplaceInline` and `ReplaceBlocks` express
every edit, and each is its own inverse, so no operation has a hand-written
undo rule that can drift from its do-rule. Commands are the rich vocabulary;
operations stay minimal.

**Transactions are atomic.** A failure halfway rolls back what already applied;
one transaction bumps the revision once, however many operations it holds.

**History stores transactions, not text.** Same tree shape as `undotree.rs`
(arena, parent/children, `current`, session redo stack, newest-branch redo) so
the two can meet in Phase 6, but a node holds the undo/redo transactions and
the selection, not a copy of the document.

### Deliberate gaps in Phase 2

These are refusals with a clear error, not silent wrong behaviour:

- Structural edits (split, merge, quote, heading change) are **top-level
  only**; `EditorError::NestedBlock` otherwise. Inline editing works at any
  depth, because blocks are found by id.
- Selections **spanning blocks** are refused (`MultiBlockSelection`). Phase 4.
- Toggling a mark with a **collapsed caret** is refused: "bold for text not yet
  typed" is caret state, not document state, and belongs with the caret in
  Phase 4.
- **No coalescing**: every `InsertText` is its own history node. Fine for
  tests, wrong for typing; Phase 6 groups by time and adjacency.
- Lists and tables are modelled and preserved but have no editing commands yet.

## 6. Risks

1. **Milkdown's role.** The hardest question in the migration, and Phase 5 is
   where it lands. Options: keep Milkdown as the view and make Rust the source
   of truth (two models in step — needs a strict one-way flow or it drifts);
   or replace the view with a thin contenteditable driven by engine state
   (loses Milkdown's IME, clipboard, mobile and a11y work, which is more than
   it looks). Prototype before choosing.
2. **IPC latency per keystroke.** A `invoke` round trip per character is not
   obviously fast enough, and diagnosing that late would be painful. Measure
   early with a real 100k-character document, and expect to need local echo
   with reconciliation rather than a naive request/response.
3. **Round-trip fidelity.** Semantic correctness first, but a diff-noisy
   serializer is a real problem for a Git/Dropbox-synced app: reflowing
   someone's Markdown on save turns one edit into a whole-file diff. Phase 3
   needs fixtures asserting that *saving an unedited document changes nothing*.
4. **Two histories.** `undotree` is on disk and load-bearing (`read_file` reads
   through it). Until Phase 6 lands, keep the engine's in-session history and
   `undotree`'s per-save versions clearly separated by scope, and do not let
   both claim ⌘Z.
5. **Front matter in two places.** Until Rust owns it, `front-matter.js` stays
   the only writer, or metadata gets reformatted on save.
6. **Codex mentions, grammar chunks and find** address text by ProseMirror
   positions or by line. They need mapping to block ids, or they break quietly.

## 7. Phases

| Phase | Work | State |
| --- | --- | --- |
| 1 | Understand the existing flow; change nothing | done (this document) |
| 2 | `editor-core`: document, inline, selection, commands, transactions, history, tests | done |
| 3 | Markdown ↔ Document, with round-trip fixtures and front matter | done |
| 4 | Multi-block selections, caret marks, list/table commands, coalescing | next |
| 5 | Tauri API (`editor_open`, `editor_apply`, `editor_state`, …) and the view decision | |
| 6 | Fold undo/redo into transactions; decide how `undotree` and engine history meet | |
| 7 | Saving/loading through the engine | |
| 8 | `chunker`, `indexer`, `search`, `export` read the document, not re-parsed text | |
| 9 | Remove the JS-side duplicates (`front-matter.js`, whole-document scans) | |

## 8. Markdown (Phase 3)

`pulldown-cmark` does the parsing; the serializer is ours. `comrak` was the
alternative and can print its own AST back to Markdown, but `Document` *is* the
AST here, so its tree would be a second model to convert through while the
serializer below still had to exist. `pulldown-cmark` is also already in the
tree (the chunker, exporter and PDF writer walk it) and its events carry byte
offsets, which is the hook an incremental reparse would want.

Front matter is split off before the parser sees the body, using the same
"looks like YAML" rule the JavaScript used, and is written back **verbatim**
from `raw` — so an untouched metadata block is never reformatted, and a
document opening with `---` as a thematic break is still a thematic break.

Three properties, tested in this order:

1. **Semantics survive.** `parse` keeps what the document says.
2. **Printing is idempotent.** One pass reaches a fixed point, so a save can
   never keep churning a file. Tested over setext headings, `+`/`1)` lists,
   reference links, autolinks, entities, indented code, tables, and more.
3. **Canonical text is untouched.** A file already in the engine's spelling
   comes back byte for byte, which is what keeps an unedited save out of a Git
   diff. The two fixtures assert this, and that an inline edit changes only the
   line it is on.

What is deliberately *not* preserved: the choice between equivalent spellings
(`*a*` becomes `_a_`, `__a__` becomes `**a**`, setext headings become ATX,
entities become the characters they name). Soft line breaks **are** kept, so
hand-wrapped paragraphs are not reflowed.

Two serializer details worth knowing, because they are easy to get wrong:

- Marks are grouped across runs, so `bold` + `bold italic` prints as
  `**bold _and italic_**` rather than `**bold** **_and italic_**` — the space
  between them is bold in the model, and only a shared wrapper says so.
- Emphasis markers can't sit against whitespace (`**a **` is not strong), so
  whitespace at the edge of a group is moved outside the markers.

Phase 4 is next: cross-block selections, caret marks, list and table commands,
and typing coalescing — the gaps listed in section 5.
