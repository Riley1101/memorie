<script>
  import { defaultValueCtx, Editor, rootCtx, editorViewCtx, serializerCtx } from '@milkdown/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { grammarPlugin } from '$lib/components/plugins/grammar';
  import { exitCodeBlockPlugin } from '$lib/components/plugins/exit-code-block';
  import { placeholderPlugin } from '$lib/components/plugins/placeholder';
  import { focusPlugin, centerCaret } from '$lib/components/plugins/focus.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { slashMenu } from '$lib/components/plugins/slash-menu.svelte.js';
  import { docRefMenu, DOC_REF_PREFIX } from '$lib/components/plugins/doc-ref.svelte.js';
  import { linkPopover, openHref } from '$lib/components/plugins/link-popover.svelte.js';
  import { smartPunctuation, autoPair } from '$lib/components/plugins/typing.js';
  import { codexMentionsPlugin, setCodexMentions } from '$lib/components/plugins/codex-mentions.js';
  import { findPlugin, findAll, buildFindRegex } from '$lib/components/plugins/find.js';
  import { marginNotesPlugin } from '$lib/components/plugins/margin-notes.js';
  import { commonmark } from '@milkdown/kit/preset/commonmark';
  import { gfm } from '@milkdown/kit/preset/gfm';
  import { changesPlugin } from '$lib/components/plugins/changes.js';
  import { revealElement } from '$lib/components/plugins/reveal.js';
  import {
    documentCounts,
    serializeIncremental,
    warmMarkdownCache,
  } from '$lib/components/plugins/block-cache.js';
  import { clipboard } from '@milkdown/kit/plugin/clipboard';
  import { appState } from '@/runes/app.svelte.js';
  import { beforeNavigate } from '$app/navigation';
  import { isMod } from '$lib/keyboard.svelte.js';
  import { openCodexEntry } from '$lib/runes/codex.svelte.js';
  import { TextSelection } from '@milkdown/kit/prose/state';

  /**
   * Selects `editorState.pendingReveal` (a project search match) in the open
   * document and scrolls it into view. Matches are counted in the rendered
   * text, so Markdown syntax can shift which one is picked; when the index
   * runs past the end the last match is used.
   * @param {import('@milkdown/kit/prose/view').EditorView} view
   */
  function revealMatch(view) {
    const pending = editorState.pendingReveal;
    editorState.pendingReveal = null;
    if (!pending?.query) return;

    const { query, options, occurrence } = pending;
    let regex;
    try {
      regex = buildFindRegex(query, options);
    } catch {
      return; // Rust-only regex syntax; opening the writing is still useful.
    }

    const { doc } = view.state;
    const matches = findAll(doc, regex, occurrence + 1);
    const found = matches[matches.length - 1];
    if (!found) return;

    const tr = view.state.tr.setSelection(TextSelection.create(doc, found.from, found.to));
    view.dispatch(tr.scrollIntoView());
    view.focus();
    const { node } = view.domAtPos(found.from);
    const el = node instanceof HTMLElement ? node : node.parentElement;
    revealElement(el);
  }

  /**
   * Collects headings with their document positions for the outline.
   * @param {import('@milkdown/kit/prose/model').Node} doc
   */
  function collectHeadings(doc) {
    /** @type {{ level: number, text: string, pos: number }[]} */
    const headings = [];
    doc.descendants((node, pos) => {
      if (node.type.name === 'heading') {
        const text = node.textContent.trim();
        if (text) headings.push({ level: node.attrs.level, text, pos });
        return false;
      }
      return !node.isTextblock;
    });
    return headings;
  }

  /**
   * @type {{
   *   defaultValue?: string,
   *   onSave?: (markdown: string) => Promise<void> | void,
   *   autofocus?: boolean,
   *   codexEntities?: import('$lib/runes/codex.svelte.js').Entity[],
   * }}
   * `codexEntities` are underlined where the text names them; empty for none.
   */
  let { defaultValue = '', onSave, autofocus = false, codexEntities = [] } = $props();

  /** @type {ReturnType<typeof setTimeout> | null} */
  let saveTimer = $state(null);
  let isReady = $state(false);

  let editorInstance = $state(null);

  /**
   * Past this many document positions (roughly characters) the editor lets
   * the browser skip offscreen blocks, and whole-document work runs less often.
   */
  const LARGE_DOC_SIZE = 100_000;
  let largeDoc = $state(false);

  /** Serializing and saving walk the whole document; wait longer when it's large. */
  const saveDelay = () => (largeDoc ? 5000 : 2000);
  /** Same for the outline and word count. */
  const scanDelay = () => (largeDoc ? 1000 : 250);

  /** @type {ReturnType<typeof setTimeout> | null} */
  let scanTimer = null;
  /** @type {import('@milkdown/kit/prose/model').Node | null} */
  let pendingScanDoc = null;

  /**
   * Word and character counts for the stats bar and today's progress.
   * Counted per block and cached, so only edited blocks are recounted.
   * @param {import('@milkdown/kit/prose/model').Node} doc
   */
  function reportCounts(doc) {
    const { words, chars } = documentCounts(doc);
    writingState.updateCounts(words, chars);
  }

  /**
   * The document as Markdown. Unchanged blocks reuse their last
   * serialization, so a save after typing in one paragraph of a novel only
   * serializes that paragraph instead of the whole book.
   * @returns {string}
   */
  function currentMarkdown() {
    if (!editorInstance) return '';
    return editorInstance.action((ctx) =>
      serializeIncremental(ctx.get(editorViewCtx).state.doc, ctx.get(serializerCtx))
    );
  }

  /** @type {ReturnType<typeof setTimeout> | null} */
  let warmTimer = null;

  /**
   * Serializes a long document's blocks in small slices while the writer
   * reads, so the first autosave doesn't serialize the whole book at once.
   * Slices stay well under a frame and yield in between.
   */
  function warmSaveCache() {
    warmTimer = null;
    if (!editorInstance || !largeDoc) return;
    const done = editorInstance.action((ctx) =>
      warmMarkdownCache(ctx.get(editorViewCtx).state.doc, ctx.get(serializerCtx), 8)
    );
    if (!done) warmTimer = setTimeout(warmSaveCache, 16);
  }

  /** @param {import('@milkdown/kit/prose/model').Node} doc */
  function onDocChanged(doc) {
    largeDoc = doc.content.size > LARGE_DOC_SIZE;
    triggerAutoSave();
    scheduleScan(doc);
  }

  /** @param {import('@milkdown/kit/prose/state').EditorState} state */
  function onSelectionChanged(state) {
    const { from, to } = state.selection;
    writingState.updateSelection(from === to ? '' : state.doc.textBetween(from, to, '\n', ' '));
  }

  /** The outline and word count both walk the whole document, so batch them while typing. */
  function scheduleScan(doc) {
    pendingScanDoc = doc;
    if (scanTimer) return;
    scanTimer = setTimeout(flushScan, scanDelay());
  }

  function flushScan() {
    if (scanTimer) clearTimeout(scanTimer);
    scanTimer = null;
    if (pendingScanDoc) {
      editorState.setHeadings(collectHeadings(pendingScanDoc));
      reportCounts(pendingScanDoc);
    }
    pendingScanDoc = null;
  }

  /**
   * Runs the save callback now. The status only flips to "saved" once the
   * backend has actually confirmed the write.
   * @param markdown {string}
   */
  async function saveNow(markdown) {
    editorState.setSaveStatus({ status: 'saving' });
    if (retryTimer) clearTimeout(retryTimer);
    retryTimer = null;
    try {
      // The backend watches the notes folder and re-indexes what was saved.
      if (onSave) await onSave(markdown);
      retryAttempt = 0;
      editorState.setSaveStatus({
        lastSaved: new Date(),
        status: 'saved',
      });
    } catch (e) {
      console.error('Auto-save failed:', e);
      editorState.setSaveStatus({ status: 'error' });
      // Keep trying quietly, backing off, so a brief hiccup (a locked file,
      // a sleeping disk) heals itself without the writer doing anything.
      const delay = RETRY_DELAYS[Math.min(retryAttempt, RETRY_DELAYS.length - 1)];
      retryAttempt += 1;
      retryTimer = setTimeout(() => {
        retryTimer = null;
        flushSave();
      }, delay);
    }
  }

  /** Background save retries after a failure: 2s, 5s, 15s, then every 30s. */
  const RETRY_DELAYS = [2000, 5000, 15000, 30000];
  let retryAttempt = 0;
  /** @type {ReturnType<typeof setTimeout> | null} */
  let retryTimer = null;

  /**
   * Trigger the auto-save mechanism with debouncing. Markdown is serialized
   * once when the timer fires, not on every pause in typing.
   */
  function triggerAutoSave() {
    if (!isReady) return;

    // Runs on every keystroke; only touch the status (and everything that
    // renders it) when it actually changes.
    if (editorState.saveStatus.status !== 'unsaved')
      editorState.setSaveStatus({ status: 'unsaved' });

    if (saveTimer) clearTimeout(saveTimer);

    saveTimer = setTimeout(() => {
      saveTimer = null;
      if (editorInstance) saveNow(currentMarkdown());
    }, saveDelay());
  }

  /**
   * Saves right away, cancelling any pending debounce. Used by ⌘S and when
   * leaving the page, so the last words are never lost.
   * @returns {Promise<void>}
   */
  function flushSave() {
    if (!editorInstance) return Promise.resolve();
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    return saveNow(currentMarkdown());
  }

  beforeNavigate(() => {
    if (saveTimer) flushSave();
  });

  /**
   * Attach the Milkdown editor to the given DOM element.
   * @param {HTMLElement} dom
   * @param {string} initialValue
   */
  function editorAttachment(dom, initialValue) {
    $effect(() => {
      if (editorInstance) return;

      // Decide before the first render, so a long document is laid out with
      // offscreen blocks skipped from the start rather than all at once and
      // then again. Markdown length is close to the document's size.
      largeDoc = initialValue.length > LARGE_DOC_SIZE;

      let editorBuilder = Editor.make()
        .config((ctx) => {
          ctx.set(rootCtx, dom);
          ctx.set(defaultValueCtx, initialValue);
        })
        .use(changesPlugin({ onDoc: onDocChanged, onSelection: onSelectionChanged }));

      editorBuilder
        .use(grammarPlugin)
        .use(exitCodeBlockPlugin)
        .use(placeholderPlugin)
        .use(focusPlugin)
        .use(autoPair)
        .use(commonmark)
        .use(gfm)
        .use(clipboard)
        .use(slashMenu)
        .use(docRefMenu)
        .use(linkPopover)
        .use(smartPunctuation)
        .use(codexMentionsPlugin)
        .use(findPlugin)
        .use(marginNotesPlugin)
        .create()
        .then((editor) => {
          if (editor) {
            editorInstance = editor;
            editorState.setEditor(editor);
            editorState.flushSave = flushSave;
            editorState.revealPending = () =>
              editor.action((ctx) => revealMatch(ctx.get(editorViewCtx)));
            if (editorState.pendingReveal) editorState.revealPending();
            editor.action((ctx) => {
              const { doc } = ctx.get(editorViewCtx).state;
              largeDoc = doc.content.size > LARGE_DOC_SIZE;
              editorState.setHeadings(collectHeadings(doc));
              writingState.resetBaseline();
              reportCounts(doc);
            });
            isReady = true;
            if (largeDoc) warmTimer = setTimeout(warmSaveCache, 1000);
            if (autofocus) {
              editor.action((ctx) => ctx.get(editorViewCtx).focus());
            }
          }
        });

      return () => {
        flushScan();
        if (retryTimer) clearTimeout(retryTimer);
        retryTimer = null;
        const dying = editorInstance;
        if (dying) {
          if (warmTimer) clearTimeout(warmTimer);
          warmTimer = null;
          editorInstance = null;
          isReady = false;
          // Milkdown holds a ProseMirror view, plugin state and DOM listeners;
          // without this they outlive every document switch and every remount
          // (font size, style flavour, restore from history).
          Promise.resolve(dying.destroy()).catch((e) =>
            console.error('Editor teardown failed:', e)
          );
          if (editorState.editor === dying) editorState.setEditor(null);
        }
        if (editorState.flushSave === flushSave) {
          editorState.flushSave = null;
          editorState.revealPending = null;
        }
        editorState.setHeadings([]);
      };
    });
  }

  // Entering focus mode jumps the caret to the middle right away; while typing
  // the focus plugin keeps it there.
  $effect(() => {
    if (!writingState.focusMode || !editorInstance) return;
    editorInstance.action((ctx) => centerCaret(ctx.get(editorViewCtx), 'instant'));
  });

  $effect(() => {
    const entities = $state.snapshot(codexEntities);
    if (!editorInstance || !isReady) return;
    editorInstance.action((ctx) => setCodexMentions(ctx.get(editorViewCtx), entities));
  });

  // ProseMirror mounts its own contenteditable div inside ours; set spellcheck
  // on it directly instead of relying on attribute inheritance, which some
  // webviews (e.g. Tauri's) don't apply consistently to contenteditable nodes.
  $effect(() => {
    const enabled = writingState.spellcheck;
    if (!editorInstance) return;
    editorInstance.action((ctx) => {
      ctx.get(editorViewCtx).dom.spellcheck = enabled;
    });
  });
</script>

<main
  class="markdown w-full"
  class:manuscript-paragraphs={writingState.paragraphStyle === 'indented'}
  class:large-doc={largeDoc}
  style="font-size: {appState.ui.fontSize}px;"
>
  <!--
    ProseMirror mounts its own focusable contenteditable inside this div, so the
    wrapper must not be a tab stop or claim a textbox role of its own. The click
    handler only intercepts links; keyboard users follow them from the caret, so
    there is no keyboard handler to pair with it.
  -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    use:editorAttachment={defaultValue}
    spellcheck={writingState.spellcheck}
    class="outline-none focus:outline-none focus-visible:outline-none"
    onclick={(e) => {
      // A codex mention opens its entry, the same ⌘-click a web link takes.
      const mention = /** @type {HTMLElement} */ (e.target).closest('[data-codex-path]');
      if (mention && isMod(e)) {
        e.preventDefault();
        const path = mention.getAttribute('data-codex-path');
        if (path) openCodexEntry(path);
        return;
      }

      const link = /** @type {HTMLElement} */ (e.target).closest('a[href]');
      if (!link) return;
      e.preventDefault();
      const href = link.getAttribute('href');
      // Doc references open on click; web links need ⌘-click so a plain
      // click can put the caret in them and bring up the link toolbar.
      if (href && (href.startsWith(DOC_REF_PREFIX) || isMod(e))) openHref(href);
    }}
  ></div>
</main>

<style>
  main :global(.ProseMirror) {
    min-height: 280px;
    text-wrap: wrap;
    outline: none;
    position: relative;
    padding-top: var(--writer-editor-pt, 0.5rem);
    padding-bottom: var(--writer-editor-pb, 50vh);
  }

  main :global(.milkdown) {
    overflow: visible !important;
  }

  /* In a book-length document the browser would otherwise lay out and
     paint every paragraph on each keystroke. Offscreen blocks are skipped
     until they scroll near the viewport; `auto` remembers each block's real
     height once it has been seen, so the scrollbar settles as you read.
     Measured in Chromium at 500k words: about 80 ms to 45 ms per key. */
  main.large-doc :global(.ProseMirror > *) {
    content-visibility: auto;
    contain-intrinsic-size: auto 6em;
  }

  main :global(.slash-menu-portal:not([data-show='true'])),
  main :global(.doc-ref-portal:not([data-show='true'])),
  main :global(.link-popover-portal:not([data-show='true'])) {
    display: none;
  }

  main :global(.codex-mention) {
    text-decoration: underline dotted;
    text-decoration-color: color-mix(in oklab, var(--primary) 55%, transparent);
    text-decoration-thickness: 1.5px;
    text-underline-offset: 0.2em;
    /* ⌘-click opens the entry, so the cursor should promise a destination. */
    cursor: pointer;
  }

  main :global(.find-match) {
    background-color: color-mix(in oklab, var(--warning) 35%, transparent);
    border-radius: 2px;
  }

  main :global(.find-match--current) {
    background-color: color-mix(in oklab, var(--warning) 80%, transparent);
    color: var(--warning-foreground);
    box-shadow: 0 0 0 1px var(--warning);
  }

  /* Text under a margin note: a light wash and an underline, in ochre so it
     never reads as a search hit (warning) or a link (primary). */
  main :global(.margin-note) {
    background-color: color-mix(in oklab, var(--binder-ochre) 14%, transparent);
    text-decoration: underline;
    text-decoration-color: color-mix(in oklab, var(--binder-ochre) 50%, transparent);
    text-decoration-thickness: 1.5px;
    text-underline-offset: 0.22em;
    border-radius: 2px;
  }

  main :global(.margin-note-active) {
    background-color: color-mix(in oklab, var(--binder-ochre) 28%, transparent);
  }

  main :global(.ProseMirror p.is-empty:first-child::before) {
    content: attr(data-placeholder);
    color: var(--writer-placeholder-color, var(--placeholder));
    pointer-events: none;
    height: 0;
    float: left;
  }

  /* Manuscript paragraph style: first-line indent, no gap between
       paragraphs, the way a typeset page reads. The first paragraph after a
       heading (or the very first paragraph) stays flush, as in print. */
  main.manuscript-paragraphs :global(.ProseMirror > p) {
    margin-block: 0;
    text-indent: 1.6em;
  }
  main.manuscript-paragraphs :global(.ProseMirror > p:first-child),
  main.manuscript-paragraphs :global(.ProseMirror > :is(h1, h2, h3, h4, h5, h6) + p) {
    text-indent: 0;
  }
</style>
