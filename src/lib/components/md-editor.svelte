<script>
  import { Input } from '@/components/ui/input/index.js';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { formatTimeAgo } from '@/utils.js';
  import Editor from './editor.svelte';
  import { invalidateAll, goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  /**
   * @type {{fileName?:string ,body?: string }}
   */
  let data = $props();
  let originalFileName = $derived(data.fileName);
  let content = $derived(data.body);

  let editedFileName = $state(data.fileName || "");

  $effect(() => {
    editedFileName = data.fileName || "";
  });

  $effect(() => {
    editorState.setName(editedFileName);
  });

  /**
   * @description Handles the save action for the editor content.
   * @param {string} content - The content to be saved.
   */
  async function onSave(content) {
    if (!editorState.editor) return;
    if (!editedFileName) return;

    if (originalFileName && editedFileName !== originalFileName) {
      // Process rename
      const oldName = originalFileName;
      const newName = editedFileName.endsWith('.md') ? editedFileName : `${editedFileName}.md`;

      if (oldName !== newName) {
        await fileManager.renameFile(oldName, newName);
        goto(resolve(`/${newName}`));
        return;
      }
    }

    // Standard save (update existing or create new if none existed)
    await fileManager.createNewFile(editedFileName, content);
    invalidateAll();
  }
</script>

<div class="p-4 pb-0">
  <div class="flex items-center w-full">
    <Input
      type="text"
      placeholder="Untitled"
      bind:value={editedFileName}
      disabled={false}
      class="h-auto border-none bg-transparent dark:bg-transparent focus-visible:ring-0 focus-visible:ring-offset-0 shadow-none text-3xl md:text-5xl font-normal mb-8 p-0 mt-8 md:mt-12 placeholder:opacity-20"
    ></Input>
    <div class="ml-auto flex items-center text-xs shrink-0 text-muted-foreground">
      <span class="lowercase first-letter:uppercase">
        {editorState.saveStatus.status}
        {formatTimeAgo(editorState.saveStatus.lastSaved)}
      </span>
    </div>
  </div>
  {#key originalFileName}
    <Editor defaultValue={content} {onSave} />
  {/key}
</div>

<style>
  :global(.ProseMirror:focus) {
    outline: none;
  }

  :global(.ProseMirror) {
    min-height: 300px;
  }
</style>
