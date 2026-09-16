<script>
  import { defaultValueCtx, Editor, rootCtx, editorViewCtx } from '@milkdown/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { grammarPlugin } from '$lib/components/plugins/grammar';
  import { exitCodeBlockPlugin } from '$lib/components/plugins/exit-code-block';
  import { placeholderPlugin } from '$lib/components/plugins/placeholder';
  import { slashMenu } from '$lib/components/plugins/slash-menu.svelte.js';
  import { docRefMenu, DOC_REF_PREFIX } from '$lib/components/plugins/doc-ref.svelte.js';
  import { memoryManager } from '$lib/runes/memory.svelte';
  import { commonmark } from '@milkdown/kit/preset/commonmark';
  import { gfm } from '@milkdown/kit/preset/gfm';
  import { listener, listenerCtx } from "@milkdown/kit/plugin/listener";
  import { clipboard } from '@milkdown/kit/plugin/clipboard'
  import { appState } from '@/runes/app.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { beforeNavigate, goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { getMarkdown } from '@milkdown/kit/utils';

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

  const DEBOUNCE_SAVE_MS = 2000;

  /**
   * Runs the save callback now. The status only flips to "saved" once the
   * backend has actually confirmed the write.
   * @param markdown {string}
   */
  async function saveNow(markdown) {
    editorState.setSaveStatus({ status: 'saving' });
    try {
      if (onSave) await onSave(markdown);
      if (configManager.config?.ai_enabled) {
        memoryManager.createDocumentContext();
      }
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
    }, DEBOUNCE_SAVE_MS);
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

      let editorBuilder = Editor
        .make()
        .config((ctx) => {
          // `updated` only fires when the doc actually changed; unlike
          // `markdownUpdated` it doesn't serialize the whole doc each time.
          ctx.get(listenerCtx).updated((ctx, doc) => {
            triggerAutoSave();
            editorState.setHeadings(collectHeadings(doc));
          });

          ctx.set(rootCtx, dom)
          ctx.set(defaultValueCtx, initialValue)
        })
        .use(listener);

      if (configManager.config?.ai_enabled) {
        editorBuilder = editorBuilder.use(grammarPlugin);
      }

      editorBuilder
        .use(exitCodeBlockPlugin)
        .use(placeholderPlugin)
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
            editor.action((ctx) => {
              editorState.setHeadings(collectHeadings(ctx.get(editorViewCtx).state.doc));
            });
            isReady = true;
            if (autofocus) {
              editor.action((ctx) => ctx.get(editorViewCtx).focus());
            }
          }
        });

      return () => {
        if (editorInstance) {
          editorInstance = null;
          isReady = false;
        }
        if (editorState.flushSave === flushSave) editorState.flushSave = null;
        editorState.setHeadings([]);
      };
    })
  }
</script>

<main
  class="markdown w-full"
  style="font-size: {appState.ui.fontSize}px;"
>
    <div
      use:editorAttachment={defaultValue}
      role="textbox"
      tabindex="0"
      spellcheck="false"
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
        color: var(--writer-placeholder-color, #9ca3af);
        pointer-events: none;
        height: 0;
        float: left;
    }
</style>