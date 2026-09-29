<script>
  /**
   * @file The editing surface for the Rust document engine.
   *
   * A `contenteditable` with no document model of its own: the engine holds the
   * document, this draws it and turns browser input intents into engine
   * commands. Every edit goes to Rust and the view is patched with what came
   * back — one paragraph, not the document.
   *
   * What lives here is only what the browser has to own: caret and selection
   * mapping, composition, and the translation of `beforeinput` into commands.
   * Everything else — what a word is, what bold means, what undo undoes — is
   * the engine's.
   */
  import { tick } from 'svelte';
  import {
    openDocument,
    openText,
    apply as applyCommands,
    patchState,
    close as closeDocument,
  } from '$lib/rust-editor.js';
  import {
    readSelection,
    writeSelection,
    blockElementOf,
    blockText,
    positionOf,
  } from '$lib/editor/dom.js';

  /** @typedef {import('$lib/rust-editor.js').EditorStateView} EditorStateView */
  /** @typedef {import('$lib/rust-editor.js').EditorBlock} EditorBlock */
  /** @typedef {import('$lib/rust-editor.js').EditorRun} EditorRun */
  /** @typedef {import('$lib/rust-editor.js').EditorCommand} EditorCommand */
  /** @typedef {import('$lib/rust-editor.js').EditorUpdate} EditorUpdate */

  /**
   * @type {{
   *   name: string,
   *   text?: string,
   *   autofocus?: boolean,
   *   spellcheck?: boolean,
   *   class?: string,
   *   onstate?: (state: EditorStateView) => void,
   *   onupdate?: (update: EditorUpdate, elapsed: number) => void,
   *   onerror?: (message: string) => void,
   *   onedit?: () => void,
   * }}
   * `name` is the writing's path, which is also the engine's session key. With
   * `text`, the document is opened from that string instead of from disk —
   * nothing is written either way; saving is the caller's business.
   * `onedit` fires after any edit, for autosave.
   */
  let {
    name,
    text = undefined,
    autofocus = false,
    spellcheck = true,
    class: className = '',
    onstate = undefined,
    onupdate = undefined,
    onerror = undefined,
    onedit = undefined,
  } = $props();

  /** @type {EditorStateView | null} */
  let doc = $state(null);
  /** @type {HTMLElement | null} */
  let root = $state(null);

  /** True while an IME holds the DOM: the view must not redraw under it. */
  let composing = $state(false);
  /** The block an IME is composing in. */
  let composingBlock = /** @type {number | null} */ (null);

  /** Input intents the engine has no command for yet, for the caller to see. */
  let unsupported = $state(/** @type {string[]} */ ([]));

  /** The document the engine has open, for a caller that wants to read it. */
  export function snapshot() {
    return doc;
  }

  /** Input intents seen but not handled, so gaps are visible rather than silent. */
  export function unsupportedInputs() {
    return unsupported;
  }

  /** Sends commands to the engine, as the caller's own UI (a toolbar) would. */
  export async function run(commands) {
    await send(Array.isArray(commands) ? commands : [commands]);
  }

  /** Focuses the surface. */
  export function focus() {
    root?.focus();
  }

  /** (Re)opens the document, discarding whatever the engine had for this name. */
  export async function reload() {
    try {
      doc =
        text === undefined
          ? await openDocument(name, { reload: true })
          : await openText(name, text);
      onstate?.(doc);
      await tick();
      if (autofocus) root?.focus();
    } catch (e) {
      onerror?.(String(e));
    }
  }

  $effect(() => {
    void name;
    void text;
    reload();
    return () => {
      // The session belongs to this surface; leaving drops it so nothing else
      // picks up edits that were never saved.
      closeDocument(name).catch(() => {});
    };
  });

  /**
   * Applies commands and patches the view with what changed.
   * @param {EditorCommand[]} commands
   */
  async function send(commands) {
    if (commands.length === 0 || !doc) return;
    const started = performance.now();
    try {
      const update = await applyCommands(name, commands);
      patchState(doc, update);
      await tick();
      // The engine decides where the caret is; the DOM is told.
      if (root && !composing) writeSelection(root, update.selection);
      onupdate?.(update, performance.now() - started);
      if (update.blocks.length > 0) onedit?.();
    } catch (e) {
      onerror?.(String(e));
    }
  }

  /** The selection the browser has, so every edit carries where it happened. */
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
      const pasted = event.dataTransfer?.getData('text/plain') ?? '';
      if (!pasted) {
        // An HTML-only payload needs an HTML → Document parser on the Rust
        // side, which isn't written yet.
        unsupported = [...new Set([...unsupported, `${type} (no text/plain)`])];
        event.preventDefault();
        return;
      }
      commands.push({ command: 'insertText', text: pasted });
    } else if (type.startsWith('delete')) {
      // The browser has already worked out what a word-wise or line-wise delete
      // covers; use its range rather than reimplementing what a word is.
      const [range] = event.getTargetRanges?.() ?? [];
      if (range && root) {
        const from = positionOf(range.startContainer, range.startOffset, root);
        const to = positionOf(range.endContainer, range.endOffset, root);
        if (from && to) commands[0] = { command: 'setSelection', anchor: from, head: to };
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
   * The IME has finished and the DOM already holds the result. Preventing that
   * would break every IME, so the engine is told what the block now says and
   * works out the smallest edit that explains it.
   */
  async function onCompositionEnd() {
    composing = false;
    const blockId = composingBlock;
    composingBlock = null;
    if (!root || blockId === null) return;
    const element = root.querySelector(`[data-block-id="${blockId}"]`);
    if (!(element instanceof HTMLElement)) return;

    const caret = readSelection(root);
    /** @type {EditorCommand[]} */
    const commands = [{ command: 'reconcileBlock', block: blockId, text: blockText(element) }];
    if (caret) commands.push({ command: 'setSelection', anchor: caret.anchor, head: caret.head });
    await send(commands);
  }

  /** @param {KeyboardEvent} event */
  function onKeydown(event) {
    const mod = event.metaKey || event.ctrlKey;
    if (!mod || composing) return;

    /** @type {EditorCommand} */
    let command;
    switch (event.key.toLowerCase()) {
      case 'b':
        command = { command: 'toggleMark', mark: 'bold' };
        break;
      case 'i':
        command = { command: 'toggleMark', mark: 'italic' };
        break;
      case 'e':
        command = { command: 'toggleMark', mark: 'code' };
        break;
      case 'z':
        command = { command: event.shiftKey ? 'redo' : 'undo' };
        break;
      default:
        return;
    }
    event.preventDefault();
    const selection = selectionCommand();
    const needsSelection = command.command === 'toggleMark';
    send(needsSelection && selection ? [selection, command] : [command]);
  }

  /** @param {EditorRun} run */
  function runClass(run) {
    return (run.marks ?? []).map((mark) => `mark-${mark}`).join(' ');
  }
</script>

{#snippet runs(list)}
  {#each list as run, index (index)}
    {#if run.type === 'text'}
      {#if run.href}
        <!-- A link in the document is content, not navigation: clicking it has
             to put the caret in it, so it isn't an anchor. -->
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
    {:else if block.level === 3}
      <h3 data-block-id={block.id}>{@render runs(block.runs ?? [])}</h3>
    {:else if block.level === 4}
      <h4 data-block-id={block.id}>{@render runs(block.runs ?? [])}</h4>
    {:else if block.level === 5}
      <h5 data-block-id={block.id}>{@render runs(block.runs ?? [])}</h5>
    {:else}
      <h6 data-block-id={block.id}>{@render runs(block.runs ?? [])}</h6>
    {/if}
  {:else if block.kind === 'quote'}
    <blockquote data-block-id={block.id}>
      {#each block.children ?? [] as child (child.id)}{@render blockView(child)}{/each}
    </blockquote>
  {:else if block.kind === 'list'}
    {#if block.ordered}
      <ol data-block-id={block.id} start={block.start ?? 1}>
        {#each block.items ?? [] as item, index (index)}
          <li class={item.checked === undefined ? '' : 'task'}>
            {#if item.checked !== undefined}
              <input type="checkbox" checked={item.checked} tabindex="-1" contenteditable="false" />
            {/if}
            {#each item.blocks as child (child.id)}{@render blockView(child)}{/each}
          </li>
        {/each}
      </ol>
    {:else}
      <ul data-block-id={block.id}>
        {#each block.items ?? [] as item, index (index)}
          <li class={item.checked === undefined ? '' : 'task'}>
            {#if item.checked !== undefined}
              <input type="checkbox" checked={item.checked} tabindex="-1" contenteditable="false" />
            {/if}
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

<div
  bind:this={root}
  class="rust-editor {className}"
  contenteditable="true"
  role="textbox"
  tabindex="0"
  aria-multiline="true"
  {spellcheck}
  onbeforeinput={onBeforeInput}
  oncompositionstart={onCompositionStart}
  oncompositionend={onCompositionEnd}
  onkeydown={onKeydown}
>
  {#each doc?.blocks ?? [] as block (block.id)}{@render blockView(block)}{/each}
</div>

<style>
  .rust-editor {
    outline: none;
  }

  .rust-editor :global(.mark-bold) {
    font-weight: 700;
  }
  .rust-editor :global(.mark-italic) {
    font-style: italic;
  }
  .rust-editor :global(.mark-strike) {
    text-decoration: line-through;
  }
  .rust-editor :global(.mark-code) {
    font-family: ui-monospace, Menlo, monospace;
    font-size: 0.9em;
    background: color-mix(in oklab, currentColor 8%, transparent);
    padding: 0 0.2em;
    border-radius: 0.2em;
  }
  .rust-editor :global(.mark-link) {
    color: var(--primary);
    text-decoration: underline;
    text-underline-offset: 0.15em;
  }
  .rust-editor :global(li.task) {
    list-style: none;
  }
  .rust-editor :global(li.task input) {
    margin-right: 0.4em;
  }
  .rust-editor :global(.raw-html) {
    opacity: 0.7;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 0.85em;
  }
</style>
