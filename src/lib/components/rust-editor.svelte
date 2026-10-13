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
    getClipboard,
    close as closeDocument,
  } from '$lib/rust-editor.js';
  import {
    readSelection,
    writeSelection,
    blockElementOf,
    blockText,
    positionOf,
    rectOfSelection,
  } from '$lib/editor/dom.js';
  import {
    commandsForCharacter,
    mightBeRule,
    backspaceThroughPair,
  } from '$lib/editor/input-rules.js';
  import { decorate, forBlock, textOfRuns } from '$lib/editor/decorations.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { openHref } from '$lib/components/plugins/link-popover.svelte.js';
  import { DOC_REF_PREFIX } from '$lib/components/plugins/doc-ref.svelte.js';
  import { isMod } from '$lib/keyboard.svelte.js';

  /** @typedef {import('$lib/rust-editor.js').EditorStateView} EditorStateView */
  /** @typedef {import('$lib/rust-editor.js').EditorBlock} EditorBlock */
  /** @typedef {import('$lib/rust-editor.js').EditorPosition} EditorPosition */
  /** @typedef {import('$lib/editor/dom.js').SelectionInfo} SelectionInfo */
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
   *   onlink?: () => Promise<string | null> | string | null,
   *   decorations?: import('$lib/editor/decorations.js').Decoration[],
   *   onselection?: (info: SelectionInfo | null) => void,
   *   placeholder?: string,
   * }}
   * `name` is the writing's path, which is also the engine's session key. With
   * `text`, the document is opened from that string instead of from disk —
   * nothing is written either way; saving is the caller's business.
   * `onedit` fires after any edit, for autosave.
   * `onlink` is asked for the URL when the writer presses ⌘K; this component
   * draws no dialogs. Without it, a URL on the clipboard is used. Return null
   * to leave the text unlinked.
   * `decorations` are ranges drawn over the text that are not in it — find
   * highlights, codex mentions, grammar squiggles. The caller computes them;
   * they never reach the engine, so they are not in the Markdown or the undo
   * history. See `$lib/editor/decorations.js`.
   * `onselection` fires when the selection settles, with everything a panel
   * floating beside the text needs to place itself — or null when the caret
   * left the surface or collapsed. This component draws no panels.
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
    onlink = undefined,
    decorations = [],
    onselection = undefined,
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

  /**
   * Where the selection is on screen and what it holds, for a caller drawing a
   * panel beside it. Null when nothing is selected or the blocks aren't
   * rendered yet.
   *
   * Read from the DOM rather than from the engine's last reply: a panel is
   * placed in answer to a selection the browser already has, and waiting for a
   * round trip to agree would put the panel a beat behind the mouse.
   */
  export function selectionInfo() {
    if (!root || !doc) return null;
    const selection = readSelection(root);
    if (!selection) return null;
    const { anchor, head } = selection;
    if (anchor.block === head.block && anchor.offset === head.offset) return null;

    const rect = rectOfSelection(root, selection);
    if (!rect) return null;
    const block = findBlock(doc.blocks, head.block);
    const whole = block?.runs ? textOfRuns(block.runs) : (block?.text ?? '');
    return {
      selection,
      rect,
      text: selectedText(),
      blockText: whole,
      block: head.block,
    };
  }

  /**
   * The selected text, read from the DOM. The engine could answer this through
   * `editor_copy`, but a panel asks at click time and a round trip for a name
   * is a round trip too many.
   */
  function selectedText() {
    const current = document.getSelection();
    return current && current.rangeCount > 0 ? current.toString() : '';
  }

  /**
   * Toggles a mark on whatever is selected now — what a floating toolbar's
   * bold button needs, without the caller having to know the command shape.
   * @param {'bold' | 'italic' | 'strike' | 'code'} mark
   */
  export async function toggleMark(mark) {
    const selection = selectionCommand();
    await send(
      selection ? [selection, { command: 'toggleMark', mark }] : [{ command: 'toggleMark', mark }]
    );
    root?.focus();
  }

  /**
   * Replaces a whole block's text with `markdown` — how an accepted AI
   * suggestion lands. The engine parses it, so a suggestion that comes back as
   * two paragraphs becomes two blocks, and the whole thing is one undo step.
   * @param {number} blockId
   * @param {string} markdown
   */
  export async function replaceBlock(blockId, markdown) {
    const block = doc ? findBlock(doc.blocks, blockId) : null;
    if (!block) return;
    const length = block.runs ? textOfRuns(block.runs).length : (block.text ?? '').length;
    await send([
      {
        command: 'setSelection',
        anchor: { block: blockId, offset: 0 },
        head: { block: blockId, offset: length },
      },
      { command: 'insertMarkdown', markdown },
    ]);
    root?.focus();
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

  $effect(() => {
    // `selectionchange` only fires on the document, never on an element, so the
    // listener cannot live on the surface in the template.
    document.addEventListener('selectionchange', onSelectionChange);
    return () => {
      document.removeEventListener('selectionchange', onSelectionChange);
      clearTimeout(syncTimer);
    };
  });

  /** The selection the engine was last told about, so a sync says it once. */
  let sentSelection = '';
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let syncTimer;

  /** @param {{ anchor: EditorPosition, head: EditorPosition }} selection */
  function selectionKey({ anchor, head }) {
    return `${anchor.block}:${anchor.offset}/${head.block}:${head.offset}`;
  }

  /**
   * Keeps the engine's idea of the caret current when the browser moves it.
   *
   * Arrow keys, a click and a drag are the browser's to handle — reimplementing
   * what a line is would be the first thing to get wrong — so the engine is
   * told about those moves here instead of hearing nothing until the next edit.
   * It matters because the caret decides what ⌘B applies to, which block focus
   * mode lights, and what a toolbar shows as active: without this they are a
   * keystroke behind the writer.
   *
   * A drag fires this per mouse move, so the work is put off until the
   * selection settles.
   */
  function onSelectionChange() {
    if (composing || !doc || !root) return;
    const selection = document.getSelection();
    // Somewhere else on the page. The engine keeps the caret it had — it is
    // still where the writer will carry on typing — but a panel floating
    // beside the text has to know it has nothing left to float beside.
    if (!selection?.anchorNode || !root.contains(selection.anchorNode)) {
      clearTimeout(syncTimer);
      onselection?.(null);
      return;
    }
    clearTimeout(syncTimer);
    syncTimer = setTimeout(syncSelection, 50);
  }

  async function syncSelection() {
    // The panel callback fires from the DOM, before and regardless of the
    // round trip: a selection that didn't change for the engine — a click
    // inside the same offsets — can still have moved on screen.
    onselection?.(selectionInfo());
    // An edit is already on its way, and its reply carries the caret the engine
    // decided on — which beats the one the browser is showing now.
    if (composing || inFlight > 0 || !doc || !root) return;
    const command = selectionCommand();
    if (!command) return;
    const key = selectionKey(command);
    if (key === sentSelection) return;
    sentSelection = key;

    const mine = generation;
    try {
      const update = await applyCommands(name, [command]);
      // Nothing is drawn and the caret is not written back: a selection changes
      // no blocks, and writing it would fight the browser for a selection it
      // already has.
      if (mine === generation && doc && inFlight === 0) patchState(doc, update);
    } catch {
      // A selection the engine won't take is not worth telling the writer
      // about: the next edit sends its own, and the view still shows the truth.
    }
  }

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
      // Writing the caret fires `selectionchange`; this is the engine's own
      // answer, so the sync has nothing left to tell it.
      sentSelection = selectionKey(update.selection);
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
      // A drop names the range it lands in, and that is the one to use: the
      // selection the DOM is still showing is the text being dragged, not
      // where the writer let go of it. The source end of a move within the
      // document is a `deleteByDrag` of its own, which the delete branch below
      // handles.
      if (type === 'insertFromDrop') commands[0] = targetSelection(event) ?? commands[0];
      // Markdown, not characters: `## Heading` off a clipboard means a heading,
      // and the engine decides whether what it parses to is blocks or a run of
      // text. A `text/plain` payload from anywhere else is still the writer's
      // best guess at what they copied, so it goes through the same door.
      commands.push({ command: 'insertMarkdown', markdown: pasted });
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

  /**
   * A copy or a cut.
   *
   * The clipboard is written asynchronously: the engine is across a bridge and
   * `clipboardData` can only be set inside the event, so the browser's own copy
   * is prevented and the Clipboard API used instead. What goes on is Markdown —
   * a paste back into Memorie keeps the formatting, and a paste anywhere else
   * reads as Markdown rather than as a DOM full of `data-block-id`.
   * @param {ClipboardEvent} event
   * @param {boolean} cut
   */
  async function onCopy(event, cut) {
    const selection = selectionCommand();
    if (!selection || !doc) return;
    const { anchor, head } = selection;
    // Nothing selected: leave the clipboard as it is rather than emptying it.
    if (anchor.block === head.block && anchor.offset === head.offset) return;

    event.preventDefault();
    try {
      // The engine hears about the caret on an edit, and a copy is not one, so
      // the selection goes over first. It changes no document state.
      await applyCommands(name, [selection]);
      const clipboard = await getClipboard(name);
      if (!clipboard.hasContent) return;
      await navigator.clipboard.writeText(clipboard.markdown);
      if (cut) await send([selection, { command: 'delete', forward: true }]);
    } catch (error) {
      onerror?.(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * ⌘K. The URL is the host's to ask for — this component draws no dialogs —
   * so it calls `onlink` and falls back to a URL sitting on the clipboard,
   * which is how a link usually arrives. Returning null leaves the text alone.
   */
  async function linkSelection() {
    const selection = selectionCommand();
    if (!selection) return;
    try {
      const url = onlink ? await onlink() : await urlOnClipboard();
      if (!url) return;
      await send([selection, { command: 'setLink', url }]);
    } catch (error) {
      onerror?.(error instanceof Error ? error.message : String(error));
    }
  }

  /** The clipboard's contents, if they read as a URL. */
  async function urlOnClipboard() {
    try {
      const text = (await navigator.clipboard.readText()).trim();
      if (!text || /\s/.test(text)) return null;
      // A bare domain is what people copy out of a browser bar as often as a
      // full URL, so both count.
      return /^[a-z][a-z0-9+.-]*:/i.test(text) || /^[\w-]+(\.[\w-]+)+/.test(text) ? text : null;
    } catch {
      return null;
    }
  }

  /**
   * Whether the caret is inside a list item, so ⇥ belongs to the engine rather
   * than to focus. Asked of the DOM rather than of the engine: the engine's
   * selection lags a caret moved with the arrow keys, and the answer decides
   * whether the key is swallowed.
   */
  function inList() {
    if (!root) return false;
    const node = document.getSelection()?.anchorNode ?? null;
    const element = node instanceof Element ? node : node?.parentElement;
    return !!element?.closest('li');
  }

  /** @param {KeyboardEvent} event */
  function onKeydown(event) {
    if (composing) return;

    // ⇥ and ⇧⇥ carry no modifier, so they come before the test for one. Only
    // a list answers them: anywhere else the key keeps its usual job of moving
    // focus out of the editor, which is what a writer reaching for it with no
    // list in sight expects, and what keyboard navigation needs.
    if (event.key === 'Tab') {
      if (!inList()) return;
      event.preventDefault();
      const selection = selectionCommand();
      const command = /** @type {EditorCommand} */ ({
        command: event.shiftKey ? 'outdentListItem' : 'indentListItem',
      });
      send(selection ? [selection, command] : [command]);
      return;
    }

    if (!isMod(event)) return;

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
      case 'k':
        // The URL has to be asked for, so this one is its own path rather than
        // a command the switch can hand on.
        event.preventDefault();
        linkSelection();
        return;
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

{#snippet runs(list, blockId)}
  <!-- Decorations split the run list before it is drawn, so a highlight that
       covers half a bold word becomes two runs rather than a nested element the
       caret mapping would have to understand. -->
  {#each decorations.length ? decorate(list, forBlock(decorations, blockId)) : list as run, index (index)}
    {#if run.type === 'text'}
      {#if run.href}
        <!-- A link in the document is content, not navigation: clicking it has
             to put the caret in it, so it isn't an anchor. -->
        <span class="{runClass(run)} {run.deco ?? ''} mark-link" data-run data-href={run.href}
          >{run.text}</span
        >
      {:else}
        <span class="{runClass(run)} {run.deco ?? ''}" data-run>{run.text}</span>
      {/if}
    {:else if run.type === 'image'}
      <img data-atom class={run.deco ?? ''} src={run.source} alt={run.alt ?? ''} />
    {:else if run.type === 'break'}
      <br data-atom />
    {:else if run.type === 'html'}
      <span data-atom class="raw-html {run.deco ?? ''}">{run.html}</span>
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
      {@render runs(block.runs ?? [], block.id)}
    </p>
  {:else if block.kind === 'heading'}
    <!-- A level outside 1–6 is not a heading the DOM has; clamping keeps a bad
         one legible instead of quietly rendering every such heading as an h6. -->
    {@const level = Math.min(Math.max(Math.trunc(block.level ?? 1), 1), 6)}
    {@const current = isCurrentBlock(block)}
    {#if level === 1}
      <h1 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [], block.id)}
      </h1>
    {:else if level === 2}
      <h2 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [], block.id)}
      </h2>
    {:else if level === 3}
      <h3 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [], block.id)}
      </h3>
    {:else if level === 4}
      <h4 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [], block.id)}
      </h4>
    {:else if level === 5}
      <h5 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [], block.id)}
      </h5>
    {:else}
      <h6 data-block-id={block.id} class:is-current-block={current}>
        {@render runs(block.runs ?? [], block.id)}
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
  oncopy={(event) => onCopy(event, false)}
  oncut={(event) => onCopy(event, true)}
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

  /* Decorations: drawn over the text, never in it. The caller names the class
     and these are the ones this component knows; an unknown `deco-*` lands on
     the span and is the caller's to style. */
  .rust-editor :global(.deco-find) {
    background: color-mix(in oklab, var(--find, #fbbf24) 45%, transparent);
    border-radius: 0.15em;
  }
  .rust-editor :global(.deco-find-current) {
    background: var(--find-current, #fbbf24);
    border-radius: 0.15em;
  }
  .rust-editor :global(.deco-mention) {
    background: color-mix(in oklab, currentColor 8%, transparent);
    border-bottom: 1px solid color-mix(in oklab, currentColor 35%, transparent);
    cursor: pointer;
  }
  /* A squiggle, not a colour change: a grammar hint has to be legible over
     text that may already be bold, linked or highlighted by a find. */
  .rust-editor :global(.deco-grammar) {
    text-decoration: underline wavy color-mix(in oklab, #ef4444 70%, transparent);
    text-decoration-skip-ink: none;
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
