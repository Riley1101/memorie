<script>
  import { Editor, rootCtx } from '@milkdown/core';
  import { commonmark } from '@milkdown/preset-commonmark';
  import { editorState } from '$lib/runes/editor.svelte';
  import { listener, listenerCtx } from '@milkdown/kit/plugin/listener';
  import { replaceAll } from '@milkdown/kit/utils';
  import { headingIdGenerator } from '@milkdown/kit/preset/commonmark';

  /**
   * @type {{ defaultValue?:string , onSave?: (markdown: string) => void}}
   */
  let props = $props();

  let defaultValue = $derived(props.defaultValue || '');

  /** @type {NodeJS.Timeout | null} */
  let saveTimer = $state(null);

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
        // try a save here
        props?.onSave?.(markdown);
        console.log('Auto-saving content:');
        editorState.setSaveStatus({
          lastSaved: new Date(),
          status: 'saved',
        });
      } catch (e) {
        console.error(e);
        editorState.setSaveStatus({
          status: 'error',
        });
      }
    }, DEBOUNCE_SAVE_MS);
  }

  $effect(() => {
    if (!editorState?.editor) return;
    editorState.editor.action(replaceAll(defaultValue));
  });

  /**
   * Attaches the Milkdown editor to the given DOM element.
   * @param {HTMLElement} dom - The DOM element to attach the editor to.
   */
  function editorAttachment(dom) {
    Editor.make()
      .config((ctx) => {
        ctx.get(listenerCtx).markdownUpdated((_ctx, markdown, _prevMarkdown) => {
          triggerAutoSave(markdown);
        });
        ctx.set(rootCtx, dom);
      })
      .use(listener)
      .use(commonmark)
      .use(headingIdGenerator)
      .create()
      .then((ed) => {
        editorState.setEditor(ed);
      });

    return () => {
      if (editorState.editor) {
        editorState.editor.destroy();
      }
    };
  }
</script>

<main class="markdown">
  <div {@attach editorAttachment}></div>
</main>
