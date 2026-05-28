<script>
  import { Input } from '@/components/ui/input/index.js';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { formatTimeAgo } from '@/utils.js';
  import Editor from './editor.svelte';
  import { invalidateAll, goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  /** Strip .md for display */
  function stripMd(name) {
    if (!name || typeof name !== 'string') return '';
    return name.replace(/\.md$/i, '').trim();
  }

  /** Ensure filename has .md for save/route */
  function ensureMd(name) {
    if (!name || !name.trim()) return '';
    const n = name.trim();
    return n.endsWith('.md') ? n : `${n}.md`;
  }

  /**
   * @type {{fileName?:string ,body?: string }}
   */
  let data = $props();
  let originalFileName = $derived(data.fileName);
  let content = $derived(data.body);

  /** Display-only title (no .md); user edits this */
  /* eslint-disable-next-line svelte/prefer-writable-derived */
  let displayTitle = $state(stripMd(data.fileName || ''));

  $effect(() => {
    displayTitle = stripMd(data.fileName || '');
  });

  $effect(() => {
    editorState.setName(ensureMd(displayTitle));
  });

  /**
   * @description Handles the save action for the editor content.
   * @param {string} content - The content to be saved.
   */
  async function onSave(content) {
    if (!editorState.editor) return;
    const filePath = ensureMd(displayTitle);
    if (!filePath) return;

    if (originalFileName && filePath !== originalFileName) {
      await fileManager.renameFile(originalFileName, filePath);
      goto(resolve(`/${filePath}`));
      return;
    }

    await fileManager.createNewFile(filePath, content);
    invalidateAll();
  }
</script>

<div class="writing-area">
  <div class="writing-area__header">
    <Input
      type="text"
      placeholder="Untitled"
      bind:value={displayTitle}
      disabled={false}
      class="writing-area__title border-none !bg-transparent h-auto !text-4xl !shadow-none px-0"
    ></Input>
    <div class="writing-area__status">
      <span class="lowercase first-letter:uppercase">
        {editorState.saveStatus.status}
        {formatTimeAgo(editorState.saveStatus.lastSaved)}
      </span>
    </div>
  </div>
  <div class="writing-area__body">
    {#key originalFileName + appState.ui.editorVersion}
      <Editor defaultValue={content} {onSave} />
    {/key}
  </div>
</div>

<style>
  .writing-area {
    padding-bottom: 0;
  }

  .writing-area__header {
    display: flex;
    align-items: flex-start;
    width: 100%;
    gap: 1rem;
    margin-bottom: var(--writer-gap-title, 0.75rem);
  }

  .writing-area__body {
    margin-top: 0;
  }

  :global(.ProseMirror:focus) {
    outline: none;
  }

  :global(.ProseMirror) {
    min-height: 280px;
  }
</style>
