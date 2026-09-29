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

**Three operations, not twelve.** `ReplaceInline`, `ReplaceBlocks` (a run of
top-level blocks) and `ReplaceBlock` (one block, at any depth) express every
edit, and each is its own inverse, so no operation has a hand-written undo rule
that can drift from its do-rule. Commands are the rich vocabulary; operations
stay minimal. `ReplaceBlock` is how a container is restructured: adding a table
row or flipping a list replaces the container wholesale, and because the blocks
inside keep their ids, a caret in a cell survives a row appearing above it.

**Transactions are atomic.** A failure halfway rolls back what already applied;
one transaction bumps the revision once, however many operations it holds.

**History stores transactions, not text.** Same tree shape as `undotree.rs`
(arena, parent/children, `current`, session redo stack, newest-branch redo) so
the two can meet in Phase 6, but a node holds the undo/redo transactions and
the selection, not a copy of the document.

**Marks for text not yet typed are caret state.** ⌘B with nothing selected
sets `Editor::pending_marks`; it changes no document state, bumps no revision
and records no history. Moving the caret drops it, and typing spends it.
`Editor::active_marks()` is what a toolbar should light up.

**A burst of typing is one undo step.** Consecutive `InsertText` commands fold
into the current history node when they carry on from where it left off, within
a 700ms window, and nothing has branched off it. Only typing coalesces —
undoing half a table insertion would be worse than an extra keypress — and
without timestamps from the host nothing coalesces at all, because guessing is
worse than an extra undo.

### Deliberate gaps after Phase 4

These are refusals with a clear error, not silent wrong behaviour:

- **Splitting, merging, quoting and heading changes are top-level only**
  (`EditorError::NestedBlock`). Inline editing works at any depth, and lists
  and tables restructure at any depth via `ReplaceBlock`; splitting a paragraph
  *inside* a list item does not yet.
- A **selection with an end inside a nested block** is refused
  (`NestedBlock`). Both ends at the top level is the case that works.
- **Indent/outdent** of list items, and removing a row or column, are not
  implemented; the commands that exist cover creating and toggling.
- **Coalescing needs a host clock.** `apply` without a timestamp records every
  keystroke separately; `apply_at` is what the Tauri layer should call.

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
| 4 | Multi-block selections, caret marks, list/table commands, coalescing | done |
| 5 | Tauri API (`editor_open`, `editor_apply`, `editor_state`, …) | done |
| 6 | Put the editing surface on the engine (spike landed; see §11) | in progress |
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

## 9. Selections, lists, tables and undo granularity (Phase 4)

A selection spanning blocks is served by joining its two ends: what follows the
selection in the last block moves onto the first, and every block between them
is removed — one transaction, two operations, invertible like any other. Ends
inside a quote or a list item are refused rather than half-handled.

Lists and tables are now editable through `ReplaceBlock`, which replaces a
container in place. `ToggleList` wraps a paragraph, flips bullets to numbers
(at any depth), or unwraps a top-level list; `ToggleTask` makes an item a task
and then ticks and unticks it; tables gain rows and columns around the caret.
The caret survives all of it because the blocks inside a container keep their
ids.

Undo granularity now matches typing: a burst folds into one history node, a
pause or a caret move starts another, and nothing but typing coalesces.

## 10. The Tauri surface (Phase 5)

Six commands, not sixty:

```text
editor_open      parse a writing, or pick up the session already open
editor_apply     apply commands, get the new state
editor_state     the current state, without changing anything
editor_markdown  what a save would write
editor_save      write it, through the same path the old save takes
editor_close     forget the session
```

One open document is one `Editor` session in `AppState.editors`, keyed by the
writing's path. `editor_open` on a document that is already open returns the
live session rather than re-parsing, so it can never quietly drop unsaved
edits; `reload: true` is the explicit way to start over.

`editor_save` and the existing `update_file` now both go through
`commands::save_writing`, so a save means the same thing whichever editor made
it: one file write, one version in the undo tree, one file-watcher event for the
indexer. That is the alternative to a second save path, and it is why the
engine can start saving without touching `undotree`, the indexer or Git.

### The wire format

`src-tauri/src/editor/wire.rs` is a format of its own, not serde on the
engine's types, because the frontend depends on its shape while the document
model should stay free to change. Two differences from the engine are
deliberate:

- **Offsets are UTF-16 code units.** That is what a JavaScript string index
  means, so `text.slice(offset)` lines up on the frontend; the engine counts
  `char`s. The conversion happens here, the one place both units are known. An
  image or a raw HTML span is one unit on both sides, since the view draws it
  as one thing. An offset landing inside a surrogate pair — which a correct
  frontend never sends — rounds down.
- **Marks are names** (`["bold", "italic"]`), which survive JSON and a version
  skew; a bitfield does not.

`src/lib/rust-editor.js` is the only place the frontend talks to the engine.

### What is *not* wired up yet, and why

Milkdown is still the editor the writer types into. This phase landed the
surface without moving the editing onto it, on purpose: switching the typing
surface is the one change in this migration that can't be made invisible, and
doing it in the same commit as the API would make both hard to review and hard
to revert.

The recommendation for Phase 6, to argue with before it is built: **keep
Milkdown as the view and make the engine the source of truth**, with a strictly
one-way flow — a keystroke goes to Rust, and the view is reconciled from the
state that comes back (revision-checked, so a stale reply is dropped). Replacing
the view with a plain `contenteditable` gives up more than it looks like:
Milkdown's IME and composition handling, clipboard, mobile behaviour and
accessibility are years of work that has nothing to do with document modelling.

The thing to measure first, before writing that reconciliation: the round-trip
cost of one keystroke on a 100k-character document. If `editor_apply` plus
reconciliation is not comfortably inside a frame, the answer is local echo in
the view with reconciliation behind it, and that changes the design enough that
it should be known up front.

## 11. Dropping Milkdown (Phase 6, in progress)

The decision is taken: **replace the view, keep `contenteditable`.** Milkdown's
remaining value was the browser-facing work — IME, caret movement through
wrapped lines, clipboard, spellcheck, a11y — while its document model, history,
input rules and Markdown handling now duplicate the engine. Keeping both models
in step indefinitely is the thing the brief said not to do, so the engine
becomes the only model and the view becomes a renderer.

`/spike-editor` is the throwaway that proves it can work. A bare
`contenteditable` with no Milkdown and no ProseMirror: every keystroke goes to
Rust through `editor_apply`, the view is redrawn from the state that comes back,
and the caret is placed where the engine says it is. The panel reports the
round-trip cost, and there is a button that builds a 100k-character document and
times 50 keystrokes against it.

Any writing can be opened in it — from the picker in its header, or with the
flask button in the editor page's top bar, which hands the open writing over.
**The spike never saves**: it has no save path, and leaving the page drops its
session, so a real document can be typed into to measure the engine against it
without risk.

The panel reports two numbers per keystroke, median and p95: what the Rust
round trip cost, and what the whole keystroke cost once the view had redrawn and
the caret was back. The second one is the one that decides whether the real view
needs local echo. `type 100` runs that measurement without hand-typing, and
`synthetic 100k` builds a 400-block document for a size the real ones may not
reach.

What to try, in the order that decides the outcome:

1. **An IME** — Japanese, pinyin, macOS long-press accents. The browser owns
   the DOM during a composition (`compositionstart` stops the view from
   redrawing), and afterwards the block's text is reported back and the engine
   works out the smallest edit that explains it.
2. **Caret movement** — up and down through the deliberately long wrapped
   paragraph, click-to-position, ⌥⌫ word delete. Word- and line-wise deletes use
   the browser's own `getTargetRanges()` rather than reimplementing what a word
   is.
3. **Paste** from a web page or a word processor. Plain text works; an
   HTML payload is listed under "not handled yet", because turning HTML into a
   `Document` is Rust-side work that hasn't been written.

### What the spike already changed in the engine

`EditorCommand::SetBlockText` — the view says what a block now reads, and the
engine computes the minimal `ReplaceInline` for it, leaving the common prefix
and suffix alone so marks and links around the change survive. Replaced text
keeps the formatting of the text it replaced (composing over a bold word stays
bold); pure insertion takes the formatting at the caret.

That diff belongs in Rust rather than in the view: it is testable there, it
keeps formatting that a JavaScript text diff would flatten, and it is the same
mechanism needed later for autocorrect, spellcheck replacements and mobile
input. On the wire it is `reconcileBlock`, which expands into two engine
commands — the first proof that a wire format earning its keep is not the same
shape as the engine's own vocabulary.

### What the real view still needs

- **Local echo**, if the round trip turns out to be too slow to render from
  Rust on every keystroke. The spike measures this before anyone builds it.
- **Decorations**: grammar results, codex mentions, find highlights. These get
  *easier* — they become ranges the engine already knows about, drawn over runs,
  instead of ProseMirror plugins mapping positions through transactions.
- **Code blocks**: the engine holds their text as text, not inline content, so
  there is nothing for a caret to address yet. The spike renders them
  read-only rather than pretending otherwise.
- **HTML paste** → `Document`, in Rust.
- The existing plugins to port: slash menu, link popover, placeholder, focus
  mode and typewriter scrolling are view-level and straightforward; smart
  punctuation and auto-pairing should become engine commands rather than view
  tricks.
