<script>
  import { defaultValueCtx, Editor, rootCtx, editorViewOptionsCtx } from '@milkdown/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { grammarPlugin } from '$lib/components/plugins/grammar';
  import { memoryManager } from '$lib/runes/memory.svelte';
  import { commonmark } from '@milkdown/kit/preset/commonmark';
  import { gfm } from '@milkdown/kit/preset/gfm';
  import { listener, listenerCtx } from "@milkdown/kit/plugin/listener";
  import { clipboard } from '@milkdown/kit/plugin/clipboard'

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
          ctx.set(editorViewOptionsCtx, {
            editable: () => editorState.editMode
          })
        })
        .use(listener)
        .use(grammarPlugin)
        .use(commonmark)
        .use(gfm)
        .use(clipboard)
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
  class="prose dark:prose-invert prose-stone prose-lg max-w-none w-full font-writer prose-p:my-2"
>
    <div use:editorAttachment={defaultValue}></div>
</main>

<style>
    main :global(.ProseMirror) {
        min-height: 200px;
        text-wrap: wrap;
        outline: none;
        position: relative;
        padding-bottom: 50vh;
    }

    main :global(.milkdown) {
        overflow: visible !important;
    }
</style>