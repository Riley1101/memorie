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
    patchBlocks,
    close as closeDocument,
  } from '$lib/rust-editor.js';
  import {
    readSelection,
    writeSelection,
    blockElementOf,
    blockText,
    positionOf,
  } from '$lib/editor/dom.js';
  import {
    commandsForCharacter,
    mightBeRule,
    backspaceThroughPair,
  } from '$lib/editor/input-rules.js';
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

  /**
   * Commands in flight. Typing faster than the round trip leaves several
   * outstanding: the engine applies them in order, but their replies can arrive
   * out of order, and an older reply must not undraw a newer one or drag the
   * caret back to where it used to be.
   */
  let inFlight = 0;
  /** The highest revision drawn so far, for the caret and the counts. */
  let drawn = -1;
  /**
   * The revision each block was last drawn at. A late reply still holds the
   * truth for the blocks it carries — the newer one only sent what *it*
   * touched — so it is folded in per block rather than dropped whole; what it
   * must not do is redraw a block someone newer has already drawn.
   * @type {Record<number, number>}
   */
  let drawnBlocks = {};
  /**
   * The revision the whole document was last drawn at. A structural reply
   * redraws every block, and block ids move with it, so nothing older than this
   * may patch a block however untouched that block looks.
   */
  let drawnFloor = -1;
  /**
   * Which document the replies in flight belong to. `reload` moves it on, so a
   * reply for the writing that was open a moment ago is dropped rather than
   * drawn over the one that is open now.
   */
  let generation = 0;

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
    const mine = (generation += 1);
    try {
      const opened =
        text === undefined
          ? await openDocument(name, { reload: true })
          : await openText(name, text);
      // Two opens can be in flight at once when the prop changes twice quickly,
      // and the first can answer last. The document that arrives late is not
      // the one the props now describe, so it is thrown away.
      if (mine !== generation) return;
      // A revision counts from zero *per document*, so the high-water mark from
      // the writing that was open before would reject every reply from this one.
      drawn = opened.revision;
      drawnFloor = opened.revision;
      drawnBlocks = {};
      doc = opened;
      onstate?.(doc);
      await tick();
      if (autofocus) root?.focus();
    } catch (e) {
      if (mine === generation) onerror?.(String(e));
    }
  }

  $effect(() => {
    // The name is captured, not read in the cleanup: by the time the cleanup
    // runs the prop may already hold the *next* document, and closing that one
    // would drop a session just opened while leaving this one behind.
    const opened = name;
    void text;
    reload();
    return () => {
      closeDocument(opened).catch(() => {});
    };
  });

  /**
   * Applies commands and patches the view with what changed.
   * @param {EditorCommand[]} commands
   */
  async function send(commands) {
    if (commands.length === 0 || !doc) return;
    const started = performance.now();
    const mine = generation;
    inFlight += 1;
    try {
      const update = await applyCommands(name, commands);
      // The document was reopened while this was in flight: its block ids mean
      // nothing against the document now on screen.
      if (mine !== generation || !doc) return;

      const stale = update.revision < drawn; // a newer reply already landed
      if (stale && update.structural) return; // its whole-document shape is behind
      // Keep the blocks this reply is still the newest word on.
      const blocks = update.blocks.filter(
        (block) => update.revision >= Math.max(drawnBlocks[block.id] ?? -1, drawnFloor)
      );
      for (const block of blocks) drawnBlocks[block.id] = update.revision;
      if (stale) {
        // Its blocks are drawn, but its caret and its counts are behind.
        if (blocks.length > 0) patchBlocks(doc, blocks);
        return;
      }
      drawn = update.revision;
      if (update.structural) {
        drawnFloor = update.revision;
        drawnBlocks = {};
      }
      patchState(doc, blocks.length === update.blocks.length ? update : { ...update, blocks });
      await tick();
      // The engine decides where the caret is, but only once nothing else is
      // still on its way: moving it under a keystroke that hasn't been answered
      // yet would put it behind the writer.
      if (root && !composing && inFlight === 1) writeSelection(root, update.selection);
      onupdate?.(update, performance.now() - started);
      if (update.blocks.length > 0) onedit?.();
    } catch (e) {
      onerror?.(String(e));
    } finally {
      inFlight -= 1;
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
   * The range a `beforeinput` says it will act on, as a selection command. The
   * browser has already worked out what a word-wise or line-wise intent covers,
   * and what an autocorrect is replacing, so its range beats reimplementing
   * what a word is. Null when it names no usable range.
   * @param {InputEvent} event
   * @returns {EditorCommand | null}
   */
  function targetSelection(event) {
    if (!root) return null;
    const [range] = event.getTargetRanges?.() ?? [];
    if (!range) return null;
    const from = positionOf(range.startContainer, range.startOffset, root);
    const to = positionOf(range.endContainer, range.endOffset, root);
    return from && to ? { command: 'setSelection', anchor: from, head: to } : null;
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
    // Worked out once: it walks the block's DOM, and the branch below both
    // tests it and uses it.
    const pair = type === 'deleteContentBackward' ? pairDelete() : null;
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
      // A soft break: the engine keeps the newline, and the serializer keeps the
      // author's wrapping. A heading is one line by definition — a newline in
      // one would come back as a heading plus a paragraph — so ⇧↵ splits the
      // block there, like ↵.
      const block = findBlock(doc?.blocks ?? [], selection.head.block);
      commands.push(
        block?.kind === 'heading'
          ? { command: 'splitBlock' }
          : { command: 'insertText', text: '\n' }
      );
    } else if (type === 'insertReplacementText') {
      // Autocorrect and the spellcheck menu: the browser names the range it is
      // replacing, so this is a delete and an insert over that range.
      const replacement = event.data ?? event.dataTransfer?.getData('text/plain') ?? '';
      const over = targetSelection(event);
      if (!replacement || !over) {
        unsupported = [...new Set([...unsupported, `${type} (no range)`])];
        event.preventDefault();
        return;
      }
      commands[0] = over;
      commands.push({ command: 'insertText', text: replacement });
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
    } else if (pair) {
      event.preventDefault();
      send(pair);
      return;
    } else if (type.startsWith('delete')) {
      // The browser has already worked out what a word-wise or line-wise delete
      // covers; use its range rather than reimplementing what a word is.
      commands[0] = targetSelection(event) ?? commands[0];
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
    const context = typingContext();
    return context ? commandsForCharacter(character, context) : null;
  }

  /** Backspace between the two halves of a pair the editor added removes both. */
  function pairDelete() {
    const context = typingContext();
    return context ? backspaceThroughPair(context) : null;
  }

  /**
   * Where the caret is and what surrounds it, for the typing rules. Null when
   * the rules shouldn't apply: no caret, a selection across blocks, or a code
   * block, which takes every character literally.
   * @returns {import('$lib/editor/input-rules.js').TypingContext | null}
   */
  function typingContext() {
    if (!root) return null;
    const selection = readSelection(root);
    if (!selection || selection.anchor.block !== selection.head.block) return null;
    const element = elementOf(selection.anchor.block);
    if (!element) return null;
    const block = findBlock(doc?.blocks ?? [], selection.anchor.block);
    if (!block || block.kind === 'codeBlock') return null;

    const [start, end] = [selection.anchor.offset, selection.head.offset].sort((a, b) => a - b);
    return {
      block: selection.anchor.block,
      start,
      end,
      text: blockText(element),
      smartPunctuation: writingState.smartPunctuation,
      autoPair: writingState.autoPair,
    };
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
    if (!isMod(event) || composing) return;

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

  /**
   * Whether the caret is in this block *or in one nested inside it*, so focus
   * mode keeps a quote, a list or a table lit while its paragraph is written
   * in. Only the top-level element is dimmed, and it is the one being asked.
   * @param {EditorBlock} block
   */
  function isCurrentBlock(block) {
    return findBlock([block], currentBlock) !== null;
  }

  /**
   * A task checkbox is the engine's to tick: the browser would flip the input
   * on its own and leave the document saying the opposite.
   * @param {Event} event
   * @param {number | undefined} blockId - The item's first block.
   */
  function onTaskToggle(event, blockId) {
    event.preventDefault();
    if (blockId === undefined) return;
    send([
      {
        command: 'setSelection',
        anchor: { block: blockId, offset: 0 },
        head: { block: blockId, offset: 0 },
      },
      { command: 'toggleTask' },
    ]);
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
    <!-- The placeholder is drawn by this paragraph's own `::before`, so the
         text has to be on the paragraph: `attr()` reads the element the
         pseudo-element belongs to, not the editor around it. -->
    <p
      data-block-id={block.id}
      data-placeholder={placeholder}
      class:is-current-block={isCurrentBlock(block)}
    >
      {@render runs(block.runs ?? [])}
    </p>
  {:else if block.kind === 'heading'}
    <!-- A level outside 1–6 is not a heading the DOM has; clamping keeps a bad
         one legible instead of quietly rendering every such heading as an h6. -->
    {@const level = Math.min(Math.max(Math.trunc(block.level ?? 1), 1), 6)}
    {@const current = isCurrentBlock(block)}
    {#if level === 1}
      <h1 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [])}
      </h1>
    {:else if level === 2}
      <h2 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [])}
      </h2>
    {:else if level === 3}
      <h3 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [])}
      </h3>
    {:else if level === 4}
      <h4 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [])}
      </h4>
    {:else if level === 5}
      <h5 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [])}
      </h5>
    {:else}
      <h6 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [])}
      </h6>
    {/if}
  {:else if block.kind === 'quote'}
    <blockquote data-block-id={block.id} class:is-current-block={isCurrentBlock(block)}>
      {#each block.children ?? [] as child (child.id)}{@render blockView(child)}{/each}
    </blockquote>
  {:else if block.kind === 'list'}
    {#if block.ordered}
      <ol
        data-block-id={block.id}
        start={block.start ?? 1}
        class:is-current-block={isCurrentBlock(block)}
      >
        {#each block.items ?? [] as item, index (index)}
          <li class={item.checked === undefined ? '' : 'task'}>
            {#if item.checked !== undefined}
              <input
                type="checkbox"
                checked={item.checked}
                tabindex="-1"
                contenteditable="false"
                onclick={(event) => onTaskToggle(event, item.blocks[0]?.id)}
              />
            {/if}
            {#each item.blocks as child (child.id)}{@render blockView(child)}{/each}
          </li>
        {/each}
      </ol>
    {:else}
      <ul data-block-id={block.id} class:is-current-block={isCurrentBlock(block)}>
        {#each block.items ?? [] as item, index (index)}
          <li class={item.checked === undefined ? '' : 'task'}>
            {#if item.checked !== undefined}
              <input
                type="checkbox"
                checked={item.checked}
                tabindex="-1"
                contenteditable="false"
                onclick={(event) => onTaskToggle(event, item.blocks[0]?.id)}
              />
            {/if}
            {#each item.blocks as child (child.id)}{@render blockView(child)}{/each}
          </li>
        {/each}
      </ul>
    {/if}
  {:else if block.kind === 'table'}
    <table data-block-id={block.id} class:is-current-block={isCurrentBlock(block)}>
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
      class:is-current-block={isCurrentBlock(block)}
      data-language={block.language ?? ''}>{block.text}</pre>
  {:else if block.kind === 'thematicBreak'}
    <hr data-block-id={block.id} class:is-current-block={isCurrentBlock(block)} />
  {:else if block.kind === 'html'}
    <div
      data-block-id={block.id}
      contenteditable="false"
      class="raw-html"
      class:is-current-block={isCurrentBlock(block)}
    >
      {block.text}
    </div>
  {:else}
    <p data-block-id={block.id} class:is-current-block={isCurrentBlock(block)}>[{block.kind}]</p>
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

  /* The engine owns the text, so the DOM has to show it verbatim: a run of
     spaces, a trailing space, and the newline of a soft break all collapse
     under the default `white-space`, which reads as the editor ignoring the
     key. `pre-wrap` keeps them and still wraps at the measure. */
  .rust-editor :global(p),
  .rust-editor :global(h1),
  .rust-editor :global(h2),
  .rust-editor :global(h3),
  .rust-editor :global(h4),
  .rust-editor :global(h5),
  .rust-editor :global(h6) {
    white-space: pre-wrap;
  }

  /* `li` and `td` are deliberately not in that list. They hold no text of their
     own — an item's and a cell's content are paragraph blocks, covered above —
     and Svelte leaves one collapsible space between an item's checkbox and its
     paragraph, which `pre-wrap` would turn into a visible indent. */

  /* An empty block has no line box, so a fresh paragraph from ↵ would be
     invisible and uncaretable. The pseudo-element is not in the DOM, so it
     costs the position mapping nothing. */
  .rust-editor :global(p:empty)::after,
  .rust-editor :global(h1:empty)::after,
  .rust-editor :global(h2:empty)::after,
  .rust-editor :global(h3:empty)::after,
  .rust-editor :global(h4:empty)::after,
  .rust-editor :global(h5:empty)::after,
  .rust-editor :global(h6:empty)::after {
    content: '\200b';
  }

  /* The placeholder sits on the first block while the document is empty. */
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
