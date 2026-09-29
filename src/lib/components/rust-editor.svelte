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
  import { commandsForCharacter, mightBeRule } from '$lib/editor/input-rules.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { openHref } from '$lib/components/plugins/link-popover.svelte.js';
  import { DOC_REF_PREFIX } from '$lib/components/plugins/doc-ref.svelte.js';
  import { isMod } from '$lib/keyboard.svelte.js';

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
   *   placeholder?: string,
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
    placeholder = 'Start writing…',
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

  /** The block the caret is in, for focus mode's dimming. */
  let currentBlock = $derived(doc?.selection.head.block ?? -1);

  /** The element for a block id, for scrolling and measuring. */
  function elementOf(blockId) {
    const element = root?.querySelector(`[data-block-id="${blockId}"]`);
    return element instanceof HTMLElement ? element : null;
  }

  /**
   * Scrolls a block into the middle of the view. Focus mode's typewriter
   * scrolling, and how the outline jumps to a heading.
   * @param {number} blockId
   * @param {ScrollBehavior} behavior
   */
  export function revealBlock(blockId, behavior = 'smooth') {
    elementOf(blockId)?.scrollIntoView({ block: 'center', behavior });
  }

  /**
   * Whether a block's element has scrolled above `threshold`, for the outline's
   * "you are here". Null when the block isn't rendered.
   * @param {number} blockId
   * @param {number} threshold
   */
  export function blockAbove(blockId, threshold) {
    const element = elementOf(blockId);
    return element ? element.getBoundingClientRect().top <= threshold : null;
  }

  /** Puts the caret at the start of a block and scrolls to it. */
  export async function goToBlock(blockId) {
    await send([
      {
        command: 'setSelection',
        anchor: { block: blockId, offset: 0 },
        head: { block: blockId, offset: 0 },
      },
    ]);
    revealBlock(blockId);
    root?.focus();
  }

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
      const character = event.data;
      const rule = mightBeRule(character) ? ruleFor(character) : null;
      if (rule) {
        event.preventDefault();
        send(rule);
        return;
      }
      commands.push({ command: 'insertText', text: character });
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

  /**
   * The commands a Markdown shortcut, a bracket or a piece of punctuation turns
   * this character into — or null to type it as it is.
   * @param {string} character
   * @returns {EditorCommand[] | null}
   */
  function ruleFor(character) {
    if (!root) return null;
    const selection = readSelection(root);
    if (!selection || selection.anchor.block !== selection.head.block) return null;
    const element = elementOf(selection.anchor.block);
    if (!element) return null;
    // A code block takes characters literally: no shortcuts inside code.
    const block = findBlock(doc?.blocks ?? [], selection.anchor.block);
    if (!block || block.kind === 'codeBlock') return null;

    const [start, end] = [selection.anchor.offset, selection.head.offset].sort((a, b) => a - b);
    return commandsForCharacter(character, {
      block: selection.anchor.block,
      start,
      end,
      text: blockText(element),
      smartPunctuation: writingState.smartPunctuation,
      autoPair: writingState.autoPair,
    });
  }

  /**
   * @param {EditorBlock[]} blocks
   * @param {number} id
   * @returns {EditorBlock | null}
   */
  function findBlock(blocks, id) {
    for (const block of blocks) {
      if (block.id === id) return block;
      const found = findBlock(
        [
          ...(block.children ?? []),
          ...(block.items ?? []).flatMap((item) => item.blocks),
          ...(block.rows ?? []).flatMap((row) => row.cells.flat()),
        ],
        id
      );
      if (found) return found;
    }
    return null;
  }

  /**
   * A link in the document is content: a plain click puts the caret in it, and
   * only a doc reference or a ⌘-click opens it — the same rule the Milkdown
   * editor follows.
   * @param {MouseEvent} event
   */
  function onClick(event) {
    const target = event.target;
    const link = target instanceof Element ? target.closest('[data-href]') : null;
    const href = link?.getAttribute('data-href');
    if (!href) return;
    if (href.startsWith(DOC_REF_PREFIX) || isMod(event)) {
      event.preventDefault();
      openHref(href);
    }
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
    <p data-block-id={block.id} class:is-current-block={block.id === currentBlock}>
      {@render runs(block.runs ?? [])}
    </p>
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
    <!-- Code is text, not runs: one text node, and the engine edits it with its
         own operation. Markdown shortcuts and smart punctuation stay out. -->
    <pre
      data-block-id={block.id}
      class:is-current-block={block.id === currentBlock}
      data-language={block.language ?? ''}>{block.text}</pre>
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
  class:focus-mode={writingState.focusMode}
  data-placeholder={placeholder}
  contenteditable="true"
  role="textbox"
  tabindex="0"
  aria-multiline="true"
  {spellcheck}
  onbeforeinput={onBeforeInput}
  oncompositionstart={onCompositionStart}
  oncompositionend={onCompositionEnd}
  onkeydown={onKeydown}
  onclick={onClick}
>
  {#each doc?.blocks ?? [] as block (block.id)}{@render blockView(block)}{/each}
</div>

<style>
  .rust-editor {
    outline: none;
  }

  /* The placeholder sits on the first block while the document is empty. */
  .rust-editor:not(:focus-within) :global(> p:first-child:last-child:empty)::before,
  .rust-editor :global(> p:first-child:last-child:empty)::before {
    content: attr(data-placeholder);
    color: var(--placeholder, color-mix(in oklab, currentColor 40%, transparent));
    pointer-events: none;
    position: absolute;
  }

  /* Focus mode: everything but the line being written fades back. */
  .rust-editor.focus-mode :global(> *) {
    transition: opacity 200ms ease;
  }

  .rust-editor.focus-mode :global(> :not(.is-current-block)) {
    opacity: 0.3;
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
