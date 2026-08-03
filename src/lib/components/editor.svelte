<script>
  import { defaultValueCtx, Editor, rootCtx } from '@milkdown/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { grammarPlugin } from '$lib/components/plugins/grammar';
  import { exitCodeBlockPlugin } from '$lib/components/plugins/exit-code-block';
  import { slashMenu } from '$lib/components/plugins/slash-menu.svelte.js';
  import { memoryManager } from '$lib/runes/memory.svelte';
  import { commonmark } from '@milkdown/kit/preset/commonmark';
  import { gfm } from '@milkdown/kit/preset/gfm';
  import { listener, listenerCtx } from "@milkdown/kit/plugin/listener";
  import { clipboard } from '@milkdown/kit/plugin/clipboard'
  import { appState } from '@/runes/app.svelte.js';
  import { openUrl } from '@tauri-apps/plugin-opener';

  /**
   * @type {{ defaultValue?: string, onSave?: (markdown: string) => void }}
   */
  let { defaultValue = '', onSave } = $props();

  /** @type {ReturnType<typeof setTimeout> | null} */
  let saveTimer = $state(null);
  let isReady = $state(false);

  let editorInstance = $state(null);

  const DEBOUNCE_SAVE_MS = 2000;

  /**
   * Trigger the auto-save mechanism with debouncing.
   * @param markdown {string}
   */
  function triggerAutoSave(markdown) {
    if (!isReady) return;

    editorState.setSaveStatus({ status: 'unsaved' });

    if (saveTimer) clearTimeout(saveTimer);

    saveTimer = setTimeout(() => {
      editorState.setSaveStatus({ status: 'saving' });
      try {
        if (onSave) onSave(markdown);
        memoryManager.createDocumentContext();
        editorState.setSaveStatus({
          lastSaved: new Date(),
          status: 'saved',
        });
      } catch (e) {
        console.error('Auto-save failed:', e);
        editorState.setSaveStatus({ status: 'error' });
      }
    }, DEBOUNCE_SAVE_MS);
  }

  /**
   * Attach the Milkdown editor to the given DOM element.
   * @param {HTMLElement} dom
   * @param {string} initialValue
   */
  function editorAttachment(dom, initialValue) {

    $effect(() => {
      if (editorInstance) return;

      Editor
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
        .use(listener)
        .use(grammarPlugin)
        .use(exitCodeBlockPlugin)
        .use(commonmark)
        .use(gfm)
        .use(clipboard)
        .use(slashMenu)
        .create()
        .then((editor) => {
          if (editor) {
            editorInstance = editor;
            editorState.setEditor(editor);
            isReady = true;
          }
        });

      return () => {
        if (editorInstance) {
          editorInstance = null;
          isReady = false;
        }
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
</style>