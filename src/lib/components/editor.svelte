<script>
  import { tick } from 'svelte';
  import { defaultValueCtx, Editor, rootCtx, editorViewOptionsCtx } from '@milkdown/core';
  import { editorViewCtx } from '@milkdown/kit/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { grammarPlugin } from '$lib/components/plugins/grammar';
  import { exitCodeBlockPlugin } from '$lib/components/plugins/exit-code-block';
  import { memoryManager } from '$lib/runes/memory.svelte';
  import { commonmark } from '@milkdown/kit/preset/commonmark';
  import { gfm } from '@milkdown/kit/preset/gfm';
  import { listener, listenerCtx } from "@milkdown/kit/plugin/listener";
  import { clipboard } from '@milkdown/kit/plugin/clipboard'
  import { appState } from '@/runes/app.svelte.js';

  /**
   * @type {{ defaultValue?: string, onSave?: (markdown: string) => void }}
   */
  let { defaultValue = '', onSave } = $props();

  /** @type {ReturnType<typeof setTimeout> | null} */
  let saveTimer = $state(null);
  let isReady = $state(false);

  let editorInstance = $state(null);
  
  /** @type {ReturnType<typeof setTimeout> | null} */
  let blurTimeout = null;
  const BLUR_DELAY_MS = 150;
  const DEBOUNCE_SAVE_MS = 2000;

  // Reactive effect to force ProseMirror view update when editMode changes
  // Also ensures focus is in editor when entering insert mode
  $effect(() => {
    const mode = editorState.editMode;
    if (editorInstance && isReady) {
      tick().then(() => {
        editorInstance.action((ctx) => {
          const view = ctx.get(editorViewCtx);
          // Force re-evaluation of editable state by updating view
          view.updateState(view.state);
          // If entering insert mode, ensure editor is focused
          if (mode && !view.hasFocus()) {
            view.focus();
          }
        });
      });
    }
  });

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
          
          // Focus listener - only clear blur timeout, don't auto-enter insert mode
          // User must press 'i' to enter insert mode
          ctx.get(listenerCtx).focus(() => {
            // Clear any pending blur timeout to prevent flicker
            if (blurTimeout) {
              clearTimeout(blurTimeout);
              blurTimeout = null;
            }
            // If not in edit mode, immediately blur to prevent interaction
            if (!editorState.editMode) {
              tick().then(() => {
                editorInstance?.action((c) => {
                  const view = c.get(editorViewCtx);
                  view.dom.blur();
                });
              });
            }
          });
          
          // Add blur listener to exit insert mode when focus leaves editor
          ctx.get(listenerCtx).blur(() => {
            // Use a small delay to prevent flicker on internal focus changes
            blurTimeout = setTimeout(() => {
              if (editorState.editMode) {
                editorState.setEditMode(false);
              }
            }, BLUR_DELAY_MS);
          });
          
          ctx.set(rootCtx, dom)
          ctx.set(defaultValueCtx, initialValue)
          ctx.set(editorViewOptionsCtx, {
            editable: () => editorState.editMode
          })
        })
        .use(listener)
        .use(grammarPlugin)
        .use(exitCodeBlockPlugin)
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
  class="markdown w-full"
  style="font-size: {appState.ui.fontSize}px;"
>
    <div 
      use:editorAttachment={defaultValue}
      role="textbox"
      tabindex="0"
      ondblclick={() => {
        if (!editorState.editMode) {
          editorState.setEditMode(true);
        }
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
</style>