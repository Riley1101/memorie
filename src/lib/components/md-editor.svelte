<script>
  import { Color } from '@tiptap/extension-text-style';
  import { ListItem } from '@tiptap/extension-list';
  import { TextStyle } from '@tiptap/extension-text-style';
  import StarterKit from '@tiptap/starter-kit';
  import { Editor } from '@tiptap/core';
  import { onMount, onDestroy } from 'svelte';
  import { Input } from '@/components/ui/input/index.js';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager } from '@/runes/fs.svelte.js';
  import { invalidateAll } from '$app/navigation';
  import { formatTimeAgo } from '@/utils.js';

  /** @type {HTMLDivElement | undefined} */
  let element = $state();

  /**
   * @type {{fileName?:string ,body?: string }}
   */
  let data = $props();
  let fileName = $derived(data.fileName);

  let content = $derived(data.body);

  /** @type {NodeJS.Timeout | null} */
  let saveTimer = $state(null);

  const DEBOUNCE_SAVE_MS = 2000;

  function triggerAutoSave() {
    editorState.setSaveStatus({
      status: 'unsaved'
    })
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      editorState.setSaveStatus({
        status: 'saving'
      })
      try {
        let contentToSave = editorState.editor?.getHTML();
        if (fileName){
          fileManager.createNewFile(fileName,contentToSave);
          invalidateAll()
        }
        editorState.setSaveStatus({
          lastSaved: new Date(),
          status: 'saved'
        })
      }catch (e){
        console.error(e)
        editorState.setSaveStatus({
          status: 'error'
        })
      }
    }, DEBOUNCE_SAVE_MS);

  }

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
      onFocus: () => {
        editorState.setEditMode(true);
      },
      onBlur: () => {
        editorState.setEditMode(false);
      },
      onUpdate:()=>{
        triggerAutoSave()
      }
    });
    editorState.setEditor(editor);
  });

  $effect(() => {
    if (editorState.editor && content !== undefined) {
      editorState.editor.commands.setContent(content);
    }
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    editorState.editor?.unmount();
  });
</script>

<div class="px-4">
  <div class="flex items-center w-full">
    <Input
      placeholder="Start writing ..."
      bind:value={fileName}
      class="h-auto border-none bg-transparent dark:bg-transparent focus-visible:ring-0 focus-visible:ring-offset-0 shadow-none md:4xl mb-4 p-0 md:text-2xl md:mt-6"
    />
    <div class="ml-auto flex items-center text-xs shrink-0 text-muted-foreground">
      <span class="lowercase first-letter:uppercase">
       {editorState.saveStatus.status} {formatTimeAgo(editorState.saveStatus.lastSaved)}
      </span>
    </div>
  </div>

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
