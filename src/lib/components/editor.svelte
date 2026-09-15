<script>
  import { defaultValueCtx, Editor, rootCtx, editorViewCtx } from '@milkdown/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { grammarPlugin } from '$lib/components/plugins/grammar';
  import { exitCodeBlockPlugin } from '$lib/components/plugins/exit-code-block';
  import { placeholderPlugin } from '$lib/components/plugins/placeholder';
  import { slashMenu } from '$lib/components/plugins/slash-menu.svelte.js';
  import { memoryManager } from '$lib/runes/memory.svelte';
  import { commonmark } from '@milkdown/kit/preset/commonmark';
  import { gfm } from '@milkdown/kit/preset/gfm';
  import { listener, listenerCtx } from "@milkdown/kit/plugin/listener";
  import { clipboard } from '@milkdown/kit/plugin/clipboard'
  import { appState } from '@/runes/app.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { beforeNavigate } from '$app/navigation';
  import { getMarkdown } from '@milkdown/kit/utils';

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
   * Trigger the auto-save mechanism with debouncing.
   * @param markdown {string}
   */
  function triggerAutoSave(markdown) {
    if (!isReady) return;

    editorState.setSaveStatus({ status: 'unsaved' });

    if (saveTimer) clearTimeout(saveTimer);

    saveTimer = setTimeout(() => {
      saveTimer = null;
      saveNow(markdown);
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
          ctx.get(listenerCtx).markdownUpdated((ctx, markdown, prevMarkdown) => {
            if (markdown !== prevMarkdown){
              triggerAutoSave(markdown)
            }
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
        .create()
        .then((editor) => {
          if (editor) {
            editorInstance = editor;
            editorState.setEditor(editor);
            editorState.flushSave = flushSave;
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
        if (href) openUrl(href);
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

    main :global(.slash-menu-portal[data-show='false']) {
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