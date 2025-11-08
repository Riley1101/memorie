<script>
  import { Color } from '@tiptap/extension-text-style';
  import { ListItem } from '@tiptap/extension-list';
  import { TextStyle } from '@tiptap/extension-text-style';
  import StarterKit from '@tiptap/starter-kit';
  import { Editor } from '@tiptap/core';
  import { onMount, onDestroy } from 'svelte';
  import MdToolbar from './md-toolbar.svelte';
  import { Input } from '@/components/ui/input/index.js';
  import { editorState } from '$lib/runes/editor.svelte.js';

  /** @type {HTMLDivElement | undefined} */
  let element = $state();

  /**
   * @type {{fileName?:string ,body?: string }}
   */
  let data = $props();

  let fileName = $derived(data.fileName);
  let content = $derived(data.body);

  onMount(() => {
    let editor = new Editor({
      element: element,
      extensions: [
        Color.configure({ types: [TextStyle.name, ListItem.name] }),
        TextStyle.configure({ types: [ListItem.name] }),
        StarterKit,
      ],
      content: content || [],
      onTransaction: ({ editor: e }) => {
        editor = e;
      },
    });
    editorState.setEditor(editor);
  });

  $effect(() => {
    if (editorState.editor && content !== undefined) {
      editorState.editor.commands.setContent(content);
    }
  });

  onDestroy(() => {
    editorState.editor?.unmount();
  });
</script>

<MdToolbar />

<div class="px-4">
  <Input
    placeholder="Start writing ..."
    bind:value={fileName}
    class="h-auto border-none bg-transparent dark:bg-transparent focus-visible:ring-0 focus-visible:ring-offset-0 shadow-none md:4xl mb-4 p-0 md:text-2xl md:mt-6"
  />

  <div bind:this={element} class="markdown rounded-lg max-w-none border-t-none w-full"></div>
</div>

<style>
  :global(.ProseMirror:focus) {
    outline: none;
  }

  :global(.ProseMirror) {
    min-height: 300px;
  }
</style>
