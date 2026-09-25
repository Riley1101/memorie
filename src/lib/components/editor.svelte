<script>
  import { defaultValueCtx, Editor, rootCtx, editorViewCtx } from '@milkdown/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { grammarPlugin } from '$lib/components/plugins/grammar';
  import { exitCodeBlockPlugin } from '$lib/components/plugins/exit-code-block';
  import { placeholderPlugin } from '$lib/components/plugins/placeholder';
  import { focusPlugin, centerCaret } from '$lib/components/plugins/focus.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { slashMenu } from '$lib/components/plugins/slash-menu.svelte.js';
  import { docRefMenu, DOC_REF_PREFIX } from '$lib/components/plugins/doc-ref.svelte.js';
  import { commonmark } from '@milkdown/kit/preset/commonmark';
  import { gfm } from '@milkdown/kit/preset/gfm';
  import { listener, listenerCtx } from '@milkdown/kit/plugin/listener';
  import { clipboard } from '@milkdown/kit/plugin/clipboard';
  import { appState } from '@/runes/app.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { beforeNavigate, goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { getMarkdown } from '@milkdown/kit/utils';
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
      let source = options.regex ? query : query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      if (options.wholeWord) source = `\\b(?:${source})\\b`;
      regex = new RegExp(source, options.caseSensitive ? 'gu' : 'giu');
    } catch {
      return; // Rust-only regex syntax; opening the writing is still useful.
    }

    const { doc } = view.state;
    /** @type {{ from: number, to: number } | null} */
    let found = null;
    let seen = 0;
    doc.descendants((node, pos) => {
      if (found && seen > occurrence) return false;
      if (!node.isTextblock) return true;
      // One placeholder character per inline leaf keeps offsets equal to positions.
      const text = doc.textBetween(pos + 1, pos + node.nodeSize - 1, undefined, '\ufffc');
      for (const m of text.matchAll(regex)) {
        if (!m[0]) continue;
        found = { from: pos + 1 + m.index, to: pos + 1 + m.index + m[0].length };
        if (seen++ === occurrence) return false;
      }
      return false;
    });
    if (!found) return;

    const tr = view.state.tr.setSelection(TextSelection.create(doc, found.from, found.to));
    view.dispatch(tr.scrollIntoView());
    view.focus();
    const { node } = view.domAtPos(found.from);
    const el = node instanceof HTMLElement ? node : node.parentElement;
    el?.scrollIntoView({ block: 'center' });
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
   * }}
   */
  let { defaultValue = '', onSave, autofocus = false } = $props();

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

  /** @param {import('@milkdown/kit/prose/model').Node} doc */
  function docText(doc) {
    return doc.textBetween(0, doc.content.size, '\n', ' ');
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
      writingState.updateDocument(docText(pendingScanDoc));
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
    try {
      // The backend watches the notes folder and re-indexes what was saved.
      if (onSave) await onSave(markdown);
      editorState.setSaveStatus({
        lastSaved: new Date(),
        status: 'saved',
      });
    } catch (e) {
      console.error('Auto-save failed:', e);
      editorState.setSaveStatus({ status: 'error' });
    }
  }

  /**
   * Trigger the auto-save mechanism with debouncing. Markdown is serialized
   * once when the timer fires, not on every pause in typing.
   */
  function triggerAutoSave() {
    if (!isReady) return;

    editorState.setSaveStatus({ status: 'unsaved' });

    if (saveTimer) clearTimeout(saveTimer);

    saveTimer = setTimeout(() => {
      saveTimer = null;
      if (editorInstance) saveNow(editorInstance.action(getMarkdown()));
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
    const markdown = editorInstance.action(getMarkdown());
    return saveNow(markdown);
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

      let editorBuilder = Editor.make()
        .config((ctx) => {
          // `updated` only fires when the doc actually changed; unlike
          // `markdownUpdated` it doesn't serialize the whole doc each time.
          ctx.get(listenerCtx).updated((ctx, doc) => {
            largeDoc = doc.content.size > LARGE_DOC_SIZE;
            triggerAutoSave();
            scheduleScan(doc);
          });
          ctx.get(listenerCtx).selectionUpdated((ctx, selection) => {
            const { from, to } = selection;
            writingState.updateSelection(
              from === to ? '' : ctx.get(editorViewCtx).state.doc.textBetween(from, to, '\n', ' ')
            );
          });

          ctx.set(rootCtx, dom);
          ctx.set(defaultValueCtx, initialValue);
        })
        .use(listener);

      if (configManager.config?.ai_enabled) {
        editorBuilder = editorBuilder.use(grammarPlugin);
      }

      editorBuilder
        .use(exitCodeBlockPlugin)
        .use(placeholderPlugin)
        .use(focusPlugin)
        .use(commonmark)
        .use(gfm)
        .use(clipboard)
        .use(slashMenu)
        .use(docRefMenu)
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
              writingState.updateDocument(docText(doc));
            });
            isReady = true;
            if (autofocus) {
              editor.action((ctx) => ctx.get(editorViewCtx).focus());
            }
          }
        });

      return () => {
        flushScan();
        if (editorInstance) {
          editorInstance = null;
          isReady = false;
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
  <div
    use:editorAttachment={defaultValue}
    role="textbox"
    tabindex="0"
    spellcheck={writingState.spellcheck}
    class="outline-none focus:outline-none focus-visible:outline-none"
    onclick={(e) => {
      const link = /** @type {HTMLElement} */ (e.target).closest('a[href]');
      if (!link) return;
      e.preventDefault();
      const href = link.getAttribute('href');
      if (!href) return;
      if (href.startsWith(DOC_REF_PREFIX)) {
        const name = decodeURIComponent(href.slice(DOC_REF_PREFIX.length));
        goto(resolve(`/${encodeURIComponent(name)}`));
        return;
      }
      openUrl(href);
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

  main :global([role='textbox']) {
    outline: none;
  }

  main :global(.milkdown) {
    overflow: visible !important;
  }

  main :global(.slash-menu-portal[data-show='false']),
  main :global(.doc-ref-portal[data-show='false']) {
    display: none;
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
