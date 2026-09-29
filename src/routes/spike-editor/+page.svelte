<script>
  /**
   * @file Throwaway editing surface for the Rust document engine.
   *
   * A bare `contenteditable` with **no Milkdown and no ProseMirror**: every
   * keystroke goes to Rust, the view is redrawn from the state that comes back,
   * and the caret is put back where the engine says it is. Nothing here is
   * meant to ship; it exists to answer the three questions that decide whether
   * Memorie can drop Milkdown (see `docs/editor-core.md`):
   *
   *   1. Does IME composition survive — Japanese, pinyin, macOS accents?
   *   2. Does the caret move sanely: up and down through wrapped lines, click
   *      to position, word-wise delete?
   *   3. Does a paste from a web page or a word processor arrive intact?
   *
   * The panel on the right reports the round-trip cost of a keystroke, which is
   * the fourth question.
   */
  import { onMount, tick } from 'svelte';
  import { openDocument, openText, apply as applyCommands, getMarkdown } from '$lib/rust-editor.js';
  import {
    readSelection,
    writeSelection,
    blockElementOf,
    blockText as domBlockText,
    positionOf,
  } from '$lib/spike/dom-selection.js';

  /** @typedef {import('$lib/rust-editor.js').EditorStateView} EditorStateView */
  /** @typedef {import('$lib/rust-editor.js').EditorBlock} EditorBlock */
  /** @typedef {import('$lib/rust-editor.js').EditorCommand} EditorCommand */

  const NAME = 'spike-editor.md';
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

  /** @type {EditorStateView | null} */
  let doc = $state(null);
  /** @type {HTMLElement | null} */
  let root = $state(null);
  let error = $state('');
  let markdown = $state('');
  let showMarkdown = $state(false);

  /** True while an IME holds the DOM; the view must not redraw. */
  let composing = $state(false);
  /** The block an IME is composing in. */
  let composingBlock = /** @type {number | null} */ (null);

  /** Round-trip times in ms, newest last. */
  let samples = $state(/** @type {number[]} */ ([]));
  let lastInputType = $state('');
  /** @type {string[]} */
  let unsupported = $state([]);
  let benchmarking = $state(false);
  let benchmark = $state('');

  function percentile(values, fraction) {
    if (values.length === 0) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * fraction))];
  }

  let median = $derived(percentile(samples, 0.5));
  let p95 = $derived(percentile(samples, 0.95));

  /**
   * Sends commands, redraws, and puts the caret back.
   * @param {EditorCommand[]} commands
   */
  async function send(commands) {
    if (commands.length === 0) return;
    const started = performance.now();
    try {
      const next = await applyCommands(NAME, commands);
      samples = [...samples.slice(-49), performance.now() - started];
      doc = next;
      error = '';
      await tick();
      // The engine decides where the caret is; the DOM is told.
      if (root && !composing) writeSelection(root, next.selection);
    } catch (e) {
      error = String(e);
    }
  }

  /** The selection the browser has, as a command, so every edit carries it. */
  function selectionCommand() {
    if (!root) return null;
    const selection = readSelection(root);
    return selection
      ? /** @type {EditorCommand} */ ({
          command: 'setSelection',
          anchor: selection.anchor,
          head: selection.head,
        })
      : null;
  }

  /**
   * Turns a browser input intent into engine commands. Everything is
   * `preventDefault`ed except composition: the engine owns the document, so the
   * browser never edits it directly.
   * @param {InputEvent} event
   */
  function onBeforeInput(event) {
    if (composing || event.isComposing) return; // the IME has the DOM
    lastInputType = event.inputType;

    const selection = selectionCommand();
    if (!selection) return;
    /** @type {EditorCommand[]} */
    const commands = [selection];

    const type = event.inputType;
    if (type === 'insertText' && event.data !== null) {
      commands.push({ command: 'insertText', text: event.data });
    } else if (type === 'insertParagraph') {
      commands.push({ command: 'splitBlock' });
    } else if (type === 'insertLineBreak') {
      // A soft break: the engine keeps the newline, and the serializer keeps
      // the author's wrapping.
      commands.push({ command: 'insertText', text: '\n' });
    } else if (type === 'insertFromPaste' || type === 'insertFromDrop') {
      const text = event.dataTransfer?.getData('text/plain') ?? '';
      if (!text) {
        // HTML-only payloads need an HTML → Document parser, which is Phase 6
        // work on the Rust side, not something to fake here.
        unsupported = [...new Set([...unsupported, `${type} (no text/plain)`])];
        event.preventDefault();
        return;
      }
      commands.push({ command: 'insertText', text });
    } else if (type.startsWith('delete')) {
      // The browser has already worked out what a word-wise or line-wise
      // delete covers; use its range rather than reimplementing it.
      const [range] = event.getTargetRanges?.() ?? [];
      if (range && root) {
        const from = positionOf(range.startContainer, range.startOffset, root);
        const to = positionOf(range.endContainer, range.endOffset, root);
        if (from && to) {
          commands[0] = { command: 'setSelection', anchor: from, head: to };
        }
      }
      commands.push({ command: 'delete', forward: type.includes('Forward') });
    } else if (type === 'historyUndo') {
      commands.length = 0;
      commands.push({ command: 'undo' });
    } else if (type === 'historyRedo') {
      commands.length = 0;
      commands.push({ command: 'redo' });
    } else {
      unsupported = [...new Set([...unsupported, type])];
      event.preventDefault();
      return;
    }

    event.preventDefault();
    send(commands);
  }

  function onCompositionStart() {
    composing = true;
    // Only the block matters: what changed inside it is worked out in Rust,
    // against the text the engine already holds.
    const block = root ? blockElementOf(document.getSelection()?.anchorNode ?? null, root) : null;
    composingBlock = block ? Number(block.dataset.blockId) : null;
  }

  /**
   * The IME has finished and the DOM already holds the result. Rather than
   * pretending to have seen the keystrokes, the engine is told what changed.
   */
  async function onCompositionEnd() {
    composing = false;
    const blockId = composingBlock;
    composingBlock = null;
    if (!root || blockId === null) return;
    const element = root.querySelector(`[data-block-id="${blockId}"]`);
    if (!(element instanceof HTMLElement)) return;

    // The DOM already holds the composed text — preventing that would break
    // every IME — so the engine is told what the block now says and works out
    // the smallest edit that explains it, keeping formatting around it.
    const caret = readSelection(root);
    /** @type {EditorCommand[]} */
    const commands = [{ command: 'reconcileBlock', block: blockId, text: domBlockText(element) }];
    if (caret) commands.push({ command: 'setSelection', anchor: caret.anchor, head: caret.head });
    await send(commands);
  }

  /** @param {KeyboardEvent} event */
  function onKeydown(event) {
    const mod = event.metaKey || event.ctrlKey;
    if (!mod || composing) return;
    const selection = selectionCommand();

    /** @type {EditorCommand} */
    let command;
    switch (event.key.toLowerCase()) {
      case 'b':
        command = { command: 'toggleMark', mark: 'bold' };
        break;
      case 'i':
        command = { command: 'toggleMark', mark: 'italic' };
        break;
      case 'z':
        command = { command: event.shiftKey ? 'redo' : 'undo' };
        break;
      case 'enter':
        command = { command: 'insertThematicBreak' };
        break;
      default:
        return;
    }
    event.preventDefault();
    const needsSelection =
      command.command === 'toggleMark' || command.command === 'insertThematicBreak';
    send(needsSelection && selection ? [selection, command] : [command]);
  }

  async function load() {
    try {
      doc = await openText(NAME, SAMPLE);
      error = '';
    } catch (e) {
      error = String(e);
    }
  }

  async function reopenFromDisk() {
    try {
      doc = await openDocument(NAME, { reload: true });
    } catch (e) {
      error = String(e);
    }
  }

  async function showTheMarkdown() {
    markdown = await getMarkdown(NAME);
    showMarkdown = true;
  }

  /** Round-trip cost on a document big enough to matter. */
  async function runBenchmark() {
    benchmarking = true;
    benchmark = '';
    try {
      const paragraph =
        'She turned the letter over twice before opening it, and the hallway light ' +
        'caught the seal in a way that made the wax look almost wet again. ';
      const source = Array.from(
        { length: 400 },
        (_, index) => `## Section ${index}\n\n${paragraph.repeat(2)}`
      ).join('\n\n');

      const big = await openText(NAME, source);
      const target = big.blocks[big.blocks.length - 2].id;
      /** @type {number[]} */
      const times = [];
      for (let i = 0; i < 50; i += 1) {
        const started = performance.now();
        await applyCommands(NAME, [
          {
            command: 'setSelection',
            anchor: { block: target, offset: 10 },
            head: { block: target, offset: 10 },
          },
          { command: 'insertText', text: 'x' },
        ]);
        times.push(performance.now() - started);
      }
      const chars = source.length;
      benchmark =
        `${chars.toLocaleString()} chars, ${big.blocks.length} blocks — ` +
        `median ${percentile(times, 0.5).toFixed(1)}ms, ` +
        `p95 ${percentile(times, 0.95).toFixed(1)}ms ` +
        `(engine only, no re-render)`;
      doc = await openText(NAME, SAMPLE);
    } catch (e) {
      benchmark = `failed: ${e}`;
    } finally {
      benchmarking = false;
    }
  }

  onMount(load);

  /** @param {import('$lib/rust-editor.js').EditorRun} run */
  function runClass(run) {
    return (run.marks ?? []).map((mark) => `mark-${mark}`).join(' ');
  }
</script>

{#snippet runs(list)}
  {#each list as run, index (index)}
    {#if run.type === 'text'}
      {#if run.href}
        <!-- A link in the document is content, not navigation: clicking it must
             put the caret in it, so it isn't an <a>. -->
        <span class="{runClass(run)} mark-link" data-run data-href={run.href}>{run.text}</span>
      {:else}
        <span class={runClass(run)} data-run>{run.text}</span>
      {/if}
    {:else if run.type === 'image'}
      <img data-atom src={run.source} alt={run.alt ?? ''} />
    {:else if run.type === 'break'}
      <br data-atom />
    {:else if run.type === 'html'}
      <span data-atom class="raw-html">{run.html}</span>
    {/if}
  {/each}
{/snippet}

{#snippet blockView(block)}
  {#if block.kind === 'paragraph'}
    <p data-block-id={block.id}>{@render runs(block.runs ?? [])}</p>
  {:else if block.kind === 'heading'}
    {#if block.level === 1}
      <h1 data-block-id={block.id}>{@render runs(block.runs ?? [])}</h1>
    {:else if block.level === 2}
      <h2 data-block-id={block.id}>{@render runs(block.runs ?? [])}</h2>
    {:else}
      <h3 data-block-id={block.id}>{@render runs(block.runs ?? [])}</h3>
    {/if}
  {:else if block.kind === 'quote'}
    <blockquote data-block-id={block.id}>
      {#each block.children ?? [] as child (child.id)}{@render blockView(child)}{/each}
    </blockquote>
  {:else if block.kind === 'list'}
    {#if block.ordered}
      <ol data-block-id={block.id}>
        {#each block.items ?? [] as item, index (index)}
          <li>
            {#if item.checked !== undefined}<input
                type="checkbox"
                checked={item.checked}
                disabled
              />{/if}
            {#each item.blocks as child (child.id)}{@render blockView(child)}{/each}
          </li>
        {/each}
      </ol>
    {:else}
      <ul data-block-id={block.id}>
        {#each block.items ?? [] as item, index (index)}
          <li>
            {#if item.checked !== undefined}<input
                type="checkbox"
                checked={item.checked}
                disabled
              />{/if}
            {#each item.blocks as child (child.id)}{@render blockView(child)}{/each}
          </li>
        {/each}
      </ul>
    {/if}
  {:else if block.kind === 'table'}
    <table data-block-id={block.id}>
      <tbody>
        {#each block.rows ?? [] as row, rowIndex (rowIndex)}
          <tr>
            {#each row.cells as cell, cellIndex (cellIndex)}
              <td
                >{#each cell as child (child.id)}{@render blockView(child)}{/each}</td
              >
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if block.kind === 'codeBlock'}
    <!-- The engine holds a code block's text as text, not inline content, so
         there is nothing here for a caret to address yet. -->
    <pre data-block-id={block.id} contenteditable="false">{block.text}</pre>
  {:else if block.kind === 'thematicBreak'}
    <hr data-block-id={block.id} />
  {:else if block.kind === 'html'}
    <div data-block-id={block.id} contenteditable="false" class="raw-html">{block.text}</div>
  {:else}
    <p data-block-id={block.id}>[{block.kind}]</p>
  {/if}
{/snippet}

<div class="spike">
  <header>
    <strong>Rust engine spike</strong>
    <span class="hint">no Milkdown, no ProseMirror — every keystroke round-trips through Rust</span>
    <button onclick={load}>reset</button>
    <button onclick={reopenFromDisk}>open from disk</button>
    <button onclick={showTheMarkdown}>markdown</button>
    <button onclick={runBenchmark} disabled={benchmarking}>
      {benchmarking ? 'benchmarking…' : 'benchmark 100k'}
    </button>
  </header>

  <div class="columns">
    <div
      bind:this={root}
      class="surface"
      contenteditable="true"
      role="textbox"
      tabindex="0"
      aria-multiline="true"
      spellcheck="true"
      onbeforeinput={onBeforeInput}
      oncompositionstart={onCompositionStart}
      oncompositionend={onCompositionEnd}
      onkeydown={onKeydown}
    >
      {#each doc?.blocks ?? [] as block (block.id)}{@render blockView(block)}{/each}
    </div>

    <aside class="panel">
      <h2>What the engine says</h2>
      <dl>
        <dt>revision</dt>
        <dd>{doc?.revision ?? '—'}</dd>
        <dt>blocks</dt>
        <dd>{doc?.blocks.length ?? 0}</dd>
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

      <h2>Round trip</h2>
      <dl>
        <dt>median</dt>
        <dd>{median.toFixed(1)}ms</dd>
        <dt>p95</dt>
        <dd>{p95.toFixed(1)}ms</dd>
        <dt>samples</dt>
        <dd>{samples.length}</dd>
        <dt>last input</dt>
        <dd>{lastInputType || '—'}</dd>
        <dt>composing</dt>
        <dd>{composing ? 'yes' : 'no'}</dd>
      </dl>
      {#if benchmark}<p class="note">{benchmark}</p>{/if}

      {#if unsupported.length}
        <h2>Not handled yet</h2>
        <ul class="note">
          {#each unsupported as type (type)}<li>{type}</li>{/each}
        </ul>
      {/if}

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
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid color-mix(in oklab, currentColor 15%, transparent);
    font-size: 0.8125rem;
  }

  .hint {
    opacity: 0.6;
    flex: 1;
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

  .surface {
    padding: 2rem clamp(1rem, 6vw, 4rem);
    overflow-y: auto;
    outline: none;
    font-size: 1.0625rem;
    line-height: 1.7;
    /* A narrow column, so wrapped lines are easy to test vertical caret
       movement against. */
    max-width: 34em;
  }

  .surface :global(h1) {
    font-size: 1.6em;
  }
  .surface :global(h2) {
    font-size: 1.25em;
  }
  .surface :global(blockquote) {
    border-left: 2px solid color-mix(in oklab, currentColor 25%, transparent);
    padding-left: 1rem;
    margin-left: 0;
    opacity: 0.85;
  }
  .surface :global(pre) {
    background: color-mix(in oklab, currentColor 7%, transparent);
    padding: 0.75rem;
    border-radius: 0.375rem;
    overflow-x: auto;
  }
  .surface :global(table) {
    border-collapse: collapse;
  }
  .surface :global(td) {
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    padding: 0.25rem 0.5rem;
  }
  .surface :global(.mark-bold) {
    font-weight: 700;
  }
  .surface :global(.mark-italic) {
    font-style: italic;
  }
  .surface :global(.mark-strike) {
    text-decoration: line-through;
  }
  .surface :global(.mark-link) {
    color: var(--primary, #2563eb);
    text-decoration: underline;
    text-underline-offset: 0.15em;
  }

  .surface :global(.mark-code) {
    font-family: ui-monospace, Menlo, monospace;
    font-size: 0.9em;
    background: color-mix(in oklab, currentColor 8%, transparent);
    padding: 0 0.2em;
    border-radius: 0.2em;
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

  .raw-html {
    opacity: 0.7;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 0.85em;
  }
</style>
