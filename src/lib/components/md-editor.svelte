<script>
  import { Input } from '@/components/ui/input/index.js';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { formatTimeAgo } from '@/utils.js';
  import Editor from './editor.svelte';
  import { invalidateAll } from '$app/navigation';

  /**
   * @type {{fileName?:string ,body?: string }}
   */
  let data = $props();
  let fileName = $derived(data.fileName);
  let content = $derived(data.body);

  $effect(() => {
    editorState.setName(fileName);
  });

  $inspect(content);

  /**
   * @description Handles the save action for the editor content.
   * @param {string} content - The content to be saved.
   */
  function onSave(content) {
    if (editorState.editor) {
      if (fileName) {
        fileManager.createNewFile(fileName, content);
        invalidateAll();
      }
    }
  }
</script>

<div class="p-4 pb-0">
  <div class="flex items-center w-full">
    <Input
      type="text"
      placeholder="Start writing ..."
      bind:value={fileName}
      class="h-auto border-none bg-transparent dark:bg-transparent focus-visible:ring-0 focus-visible:ring-offset-0 shadow-none md:4xl mb-4 p-0 md:text-2xl md:mt-6"
    ></Input>
    <div class="ml-auto flex items-center text-xs shrink-0 text-muted-foreground">
      <span class="lowercase first-letter:uppercase">
        {editorState.saveStatus.status}
        {formatTimeAgo(editorState.saveStatus.lastSaved)}
      </span>
    </div>
  </div>
  <Editor defaultValue={content} {onSave} />
</div>

<style>
  :global(.ProseMirror:focus) {
    outline: none;
  }

  :global(.ProseMirror) {
    min-height: 300px;
  }
</style>
