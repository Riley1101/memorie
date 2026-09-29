<script>
  /**
   * @file Harness for the Rust engine's editing surface.
   *
   * The surface itself is `rust-editor.svelte`, the component the app uses;
   * this page is the instrumentation around it — a document picker, a synthetic
   * document of a size the real ones may not reach, an auto-typer, and the
   * timings that say where a keystroke's cost goes.
   *
   * The questions it exists to answer (see `docs/editor-core.md`):
   *
   *   1. Does IME composition survive — Japanese, pinyin, macOS accents?
   *   2. Does the caret move sanely: up and down through wrapped lines, click
   *      to position, word-wise delete?
   *   3. Does a paste from a web page or a word processor arrive intact?
   *   4. What does a keystroke cost, and which part of it?
   *
   * Nothing here saves. Leaving the page drops the engine's session, so a real
   * writing can be typed into to measure against without risk.
   */
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { spikeTarget } from '$lib/editor/spike-target.svelte.js';
  import { getMarkdown } from '$lib/rust-editor.js';
  import RustEditor from '$lib/components/rust-editor.svelte';

  /** @typedef {import('$lib/rust-editor.js').EditorStateView} EditorStateView */
  /** @typedef {import('$lib/rust-editor.js').EditorUpdate} EditorUpdate */
  /** @typedef {import('$lib/rust-editor.js').EditorBlock} EditorBlock */

  const SCRATCH = 'spike-editor.md';
  const SAMPLE = `# The spike

A paragraph with **bold**, _italic_, \`code\` and a [link](notes.md). Type into
it, paste into it, and try an IME.

## Things to try

- delete a word with ⌥⌫
- press ⌘B with a selection, and with none
- move up and down through this long wrapped line, which should keep going for
  long enough to wrap at least twice in the column so that vertical caret
  movement has something to be tested against, because that is the movement a
  custom editor usually gets wrong first

> A quote, to check nested blocks.

| Name | Role |
| :--- | ---: |
| Ada | Writer |
`;

  /** The writing to open, handed over by the editor page or picked here. */
  let requested = $state(spikeTarget.name ?? page.url.searchParams.get('doc'));
  /** Text to open instead of a file: the sample, or a synthetic document. */
  let scratchText = $state(SAMPLE);

  let openName = $derived(requested ?? SCRATCH);
  let openText = $derived(requested ? undefined : scratchText);

  /** @type {RustEditor | null} */
  let surface = $state(null);
  /** @type {EditorStateView | null} */
  let doc = $state(null);
  let error = $state('');
  let markdown = $state('');
  let showMarkdown = $state(false);

  /** Engine time in ms, as Rust measured itself. Newest last. */
  let engineTimes = $state(/** @type {number[]} */ ([]));
  /** The same keystrokes measured to the end: bridge, redraw and caret. */
  let wholeTimes = $state(/** @type {number[]} */ ([]));
  let typing = $state(false);
  let report = $state('');

  function percentile(values, fraction) {
    if (values.length === 0) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * fraction))];
  }

  let engineMedian = $derived(percentile(engineTimes, 0.5));
  let engineP95 = $derived(percentile(engineTimes, 0.95));
  let wholeMedian = $derived(percentile(wholeTimes, 0.5));
  let wholeP95 = $derived(percentile(wholeTimes, 0.95));

  /** @param {EditorBlock} block */
  function blockChars(block) {
    const own =
      (block.runs ?? []).reduce((sum, run) => sum + (run.text?.length ?? 1), 0) +
      (block.text?.length ?? 0);
    const nested = [
      ...(block.children ?? []),
      ...(block.items ?? []).flatMap((item) => item.blocks),
      ...(block.rows ?? []).flatMap((row) => row.cells.flat()),
    ];
    return own + nested.reduce((sum, child) => sum + blockChars(child), 0);
  }

  let charCount = $derived((doc?.blocks ?? []).reduce((total, b) => total + blockChars(b), 0));

  /**
   * The ids of every block a caret can go in, in document order.
   * @param {EditorBlock[]} blocks
   * @returns {number[]}
   */
  function textBlocks(blocks) {
    /** @type {number[]} */
    const out = [];
    for (const block of blocks) {
      if (block.kind === 'paragraph' || block.kind === 'heading') out.push(block.id);
      out.push(
        ...textBlocks([
          ...(block.children ?? []),
          ...(block.items ?? []).flatMap((item) => item.blocks),
          ...(block.rows ?? []).flatMap((row) => row.cells.flat()),
        ])
      );
    }
    return out;
  }

  /** @param {EditorStateView} next */
  function onState(next) {
    doc = next;
    engineTimes = [];
    wholeTimes = [];
    report = '';
    error = '';
  }

  /**
   * @param {EditorUpdate} update
   * @param {number} elapsed - Round trip, redraw and caret, in ms.
   */
  function onUpdate(update, elapsed) {
    engineTimes = [...engineTimes.slice(-199), update.engineMicros / 1000];
    wholeTimes = [...wholeTimes.slice(-199), elapsed];
  }

  /** @param {string} name */
  function openWriting(name) {
    requested = name || null;
    spikeTarget.name = requested;
    if (!name) scratchText = SAMPLE;
  }

  /** A synthetic document, for a size the real writings may not reach. */
  /** @param {number} sections */
  function loadSynthetic(sections) {
    const paragraph =
      'She turned the letter over twice before opening it, and the hallway light ' +
      'caught the seal in a way that made the wax look almost wet again. ';
    requested = null;
    spikeTarget.name = null;
    scratchText = Array.from(
      { length: sections },
      (_, index) => `## Section ${index}\n\n${paragraph.repeat(2)}`
    ).join('\n\n');
  }

  /**
   * Types `count` characters through the whole path, so the numbers don't
   * depend on how fast anybody types.
   * @param {number} count
   */
  async function stress(count) {
    if (!surface || !doc) return;
    typing = true;
    report = '';
    try {
      const candidates = textBlocks(doc.blocks);
      const target = candidates[Math.floor(candidates.length / 2)];
      if (target === undefined) throw new Error('nothing to type into');

      await surface.run([
        {
          command: 'setSelection',
          anchor: { block: target, offset: 0 },
          head: { block: target, offset: 0 },
        },
      ]);
      engineTimes = [];
      wholeTimes = [];

      for (let i = 0; i < count; i += 1) {
        await surface.run([{ command: 'insertText', text: i % 12 === 11 ? ' ' : 'x' }]);
      }

      report =
        `${count} keystrokes into ${charCount.toLocaleString()} chars / ` +
        `${doc.blocks.length} blocks — engine ${percentile(engineTimes, 0.5).toFixed(2)}ms ` +
        `(p95 ${percentile(engineTimes, 0.95).toFixed(2)}ms), ` +
        `whole keystroke ${percentile(wholeTimes, 0.5).toFixed(1)}ms ` +
        `(p95 ${percentile(wholeTimes, 0.95).toFixed(1)}ms)`;
    } catch (e) {
      report = `failed: ${e}`;
    } finally {
      typing = false;
    }
  }

  async function showTheMarkdown() {
    markdown = await getMarkdown(openName);
    showMarkdown = true;
  }

  onMount(() => {
    if (!fileManager.hasLoadedFiles) fileManager.getRecents();
  });
</script>

<div class="spike">
  <header>
    <strong>Rust engine spike</strong>
    <!-- Any writing can be opened here to measure the engine against it.
         Nothing is ever written back: the spike has no save. -->
    <select
      aria-label="Document to open"
      value={requested ?? ''}
      onchange={(event) => openWriting(event.currentTarget.value)}
    >
      <option value="">— sample document —</option>
      {#each fileManager.files as file (file.name)}
        <option value={file.name}>{file.name}</option>
      {/each}
    </select>
    <span class="hint">
      {requested ? 'edits here are never saved' : 'no Milkdown, no ProseMirror'}
    </span>
    <button onclick={() => surface?.reload()}>reload</button>
    <button onclick={showTheMarkdown}>markdown</button>
    <button onclick={() => loadSynthetic(400)} disabled={typing}>synthetic 100k</button>
    <button onclick={() => stress(100)} disabled={typing}>
      {typing ? 'typing…' : 'type 100'}
    </button>
  </header>

  <div class="columns">
    <div class="surface-wrap">
      <RustEditor
        bind:this={surface}
        name={openName}
        text={openText}
        autofocus
        class="surface"
        onstate={onState}
        onupdate={onUpdate}
        onerror={(message) => (error = message)}
      />
    </div>

    <aside class="panel">
      <h2>What the engine says</h2>
      <dl>
        <dt>revision</dt>
        <dd>{doc?.revision ?? '—'}</dd>
        <dt>blocks</dt>
        <dd>{doc?.blocks.length ?? 0}</dd>
        <dt>characters</dt>
        <dd>{charCount.toLocaleString()}</dd>
        <dt>words</dt>
        <dd>{doc?.wordCount ?? 0}</dd>
        <dt>selection</dt>
        <dd>
          {#if doc}
            {doc.selection.anchor.block}:{doc.selection.anchor.offset} →
            {doc.selection.head.block}:{doc.selection.head.offset}
          {/if}
        </dd>
        <dt>active marks</dt>
        <dd>{doc?.activeMarks.join(', ') || 'none'}</dd>
        <dt>undo / redo</dt>
        <dd>{doc?.canUndo ? 'yes' : 'no'} / {doc?.canRedo ? 'yes' : 'no'}</dd>
      </dl>

      <h2>Per keystroke</h2>
      <dl>
        <dt>engine</dt>
        <dd>{engineMedian.toFixed(2)} / {engineP95.toFixed(2)}ms</dd>
        <dt>whole</dt>
        <dd>{wholeMedian.toFixed(1)} / {wholeP95.toFixed(1)}ms</dd>
        <dt>samples</dt>
        <dd>{engineTimes.length}</dd>
      </dl>
      <p class="note">
        median / p95. “Engine” is Rust measuring itself; “whole” adds the bridge, the redraw and the
        caret — so the gap between them is the part the engine can't fix.
      </p>
      {#if report}<p class="note">{report}</p>{/if}

      {#if error}<p class="error">{error}</p>{/if}
    </aside>
  </div>

  {#if showMarkdown}
    <section class="markdown">
      <header>
        <strong>What a save would write</strong>
        <button onclick={() => (showMarkdown = false)}>close</button>
      </header>
      <pre>{markdown}</pre>
    </section>
  {/if}
</div>

<style>
  .spike {
    display: flex;
    flex-direction: column;
    height: 100vh;
    font-family: system-ui, sans-serif;
    background: var(--background, #fff);
    color: var(--foreground, #111);
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem;
    /* The app's macOS overlay title bar is a fixed strip across the top of the
       window; without this the header sits under it and can't be seen or
       clicked. Padding, not a z-index, because that strip is the drag region. */
    padding: calc(0.75rem + var(--titlebar-height, 0px)) 1rem 0.75rem;
    border-bottom: 1px solid color-mix(in oklab, currentColor 15%, transparent);
    font-size: 0.8125rem;
  }

  .hint {
    opacity: 0.6;
    flex: 1;
  }

  select {
    font: inherit;
    font-size: 0.75rem;
    max-width: 16rem;
    padding: 0.2rem 0.3rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 0.375rem;
    background: transparent;
    color: inherit;
  }

  button {
    font: inherit;
    font-size: 0.75rem;
    padding: 0.25rem 0.6rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 0.375rem;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 17rem;
    flex: 1;
    min-height: 0;
  }

  .surface-wrap {
    overflow-y: auto;
    padding: 2rem clamp(1rem, 6vw, 4rem);
  }

  /* A narrow column, so wrapped lines are easy to test vertical caret movement
     against. */
  .surface-wrap :global(.surface) {
    max-width: 34em;
    font-size: 1.0625rem;
    line-height: 1.7;
  }

  .surface-wrap :global(h1) {
    font-size: 1.6em;
  }
  .surface-wrap :global(h2) {
    font-size: 1.25em;
  }
  .surface-wrap :global(blockquote) {
    border-left: 2px solid color-mix(in oklab, currentColor 25%, transparent);
    padding-left: 1rem;
    margin-left: 0;
    opacity: 0.85;
  }
  .surface-wrap :global(pre) {
    background: color-mix(in oklab, currentColor 7%, transparent);
    padding: 0.75rem;
    border-radius: 0.375rem;
    overflow-x: auto;
  }
  .surface-wrap :global(table) {
    border-collapse: collapse;
  }
  .surface-wrap :global(td) {
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    padding: 0.25rem 0.5rem;
  }

  .panel {
    border-left: 1px solid color-mix(in oklab, currentColor 15%, transparent);
    padding: 1rem;
    overflow-y: auto;
    font-size: 0.75rem;
  }

  .panel h2 {
    font-size: 0.6875rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    opacity: 0.6;
    margin: 1.25rem 0 0.5rem;
  }

  .panel h2:first-child {
    margin-top: 0;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.25rem 0.75rem;
    margin: 0;
  }

  dt {
    opacity: 0.6;
  }

  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .note {
    opacity: 0.75;
    line-height: 1.5;
  }

  .error {
    color: #b00020;
    line-height: 1.5;
  }

  .markdown {
    border-top: 1px solid color-mix(in oklab, currentColor 15%, transparent);
    max-height: 40vh;
    overflow: auto;
  }

  .markdown pre {
    margin: 0;
    padding: 1rem;
    font-size: 0.75rem;
    white-space: pre-wrap;
  }
</style>
