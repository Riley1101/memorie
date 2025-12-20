<script>
  import { Editor, rootCtx } from '@milkdown/core';
  import { commonmark } from '@milkdown/preset-commonmark';
  import { gfm } from '@milkdown/kit/preset/gfm';
  import { editorState } from '$lib/runes/editor.svelte';
  import { listener, listenerCtx } from '@milkdown/kit/plugin/listener';
  import { replaceAll, getMarkdown } from '@milkdown/kit/utils';
  import { headingIdGenerator } from '@milkdown/kit/preset/commonmark';
  import { clipboard } from '@milkdown/kit/plugin/clipboard';
  import { untrack } from 'svelte';

  /**
   * @type {{ defaultValue?: string, onSave?: (markdown: string) => void }}
   */
  let { defaultValue = '', onSave } = $props();

  /** @type {ReturnType<typeof setTimeout> | null} */
  let saveTimer = $state(null);
  let isReady = $state(false);

  const DEBOUNCE_SAVE_MS = 2000;

  /**
   * Triggers an auto-save operation after a debounce period.
   * @param {string} markdown - The current markdown content.
   */
  function triggerAutoSave(markdown) {
    editorState.setSaveStatus({
      status: 'unsaved',
    });

    if (saveTimer) clearTimeout(saveTimer);

    saveTimer = setTimeout(() => {
      editorState.setSaveStatus({
        status: 'saving',
      });

      try {
        if (onSave) {
          onSave(markdown);
        }

        editorState.setSaveStatus({
          lastSaved: new Date(),
          status: 'saved',
        });
      } catch (e) {
        console.error('Auto-save failed:', e);
        editorState.setSaveStatus({
          status: 'error',
        });
      }
    }, DEBOUNCE_SAVE_MS);
  }

  /**
   * Updates the editor content only if it differs from the current content.
   * This prevents cursor jumping and infinite loops.
   */
  $effect(() => {
    const newValue = defaultValue;

    untrack(() => {
      if (!editorState?.editor || !isReady) return;

      try {
        const currentMarkdown = editorState.editor.action(getMarkdown());
        if (currentMarkdown !== newValue) {
          editorState.editor.action(replaceAll(newValue));
        }
      } catch (e) {
        console.warn('Editor view not ready for update:', e);
      }
    });
  });

  /**
   * Svelte Action to attach the Milkdown editor.
   * @param {HTMLElement} dom - The DOM element to attach the editor to.
   */
  function editorAttachment(dom) {
    /**
     * @type {import('@milkdown/core').Editor | null}
     */
    let editorInstance;

    Editor.make()
      .config((ctx) => {
        ctx.set(rootCtx, dom);
        ctx.get(listenerCtx).markdownUpdated((_ctx, markdown, _prevMarkdown) => {
          triggerAutoSave(markdown);
        });
      })
      .use(listener)
      .use(commonmark)
      .use(headingIdGenerator)
      .use(gfm)
      .use(clipboard)
      .create()
      .then((ed) => {
        editorInstance = ed;
        editorState.setEditor(ed);
        ed.action(replaceAll(defaultValue));
        isReady = true;
      });

    return {
      destroy() {
        isReady = false;
        if (editorInstance) {
          editorInstance.destroy();
        }
        if (saveTimer) clearTimeout(saveTimer);
      },
    };
  }
</script>

<main class="prose dark:prose-invert prose-stone prose-base max-w-none w-full font-writer">
  <div use:editorAttachment></div>
</main>

<style>
  main :global(.ProseMirror) {
    min-height: 200px;
    outline: none;
  }
</style>
