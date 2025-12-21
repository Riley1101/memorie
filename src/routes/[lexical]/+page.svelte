<script>
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import MarkdownEditor from '$lib/components/md-editor.svelte';
  import EditorCommandbar from '$lib/components/editor-commandbar.svelte';
  import EditorHistory from '$lib/components/editor-history.svelte';
  import EditorOutline from '$lib/components/editor-outline.svelte';
  import Button from '$lib/components/ui/button/button.svelte';
  import HomeIcon from '@lucide/svelte/icons/home';
  import { appState } from '$lib/runes/app.svelte.js';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { editorState } from '$lib/runes/editor.svelte';

  let { data } = $props();

  /**
   * @typedef {Object} RecordPointer
   * @property {string} tb - The table name (e.g., 'chunk', 'documents').
   * @property {{String:string}} id - The record identifier wrapper.
   */

  /**
   * @typedef {Object} ChunkItem
   * @property {RecordPointer} id - The unique identifier for this chunk.
   * @property {RecordPointer} parent - The reference to the parent document.
   * @property {string} content - The text content of the paragraph.
   * @property {number} sequence - The order of the paragraph (0-indexed).
   * @property {string} content_hash - A hash string for the content.
   * @property {?Object} grammar_check - Grammar check results (nullable).
   * @property {boolean} is_dirty - Indicates if the content has been modified.
   */

  /**
   * @typedef {Object} Node
   * @property {string} content - The text content of this history state.
   * @property {number | null} parent - The index in the `nodes` array of the parent node.
   * @property {number[]} children - An array of indices for all child nodes.
   */

  /**
   * @typedef {Object} History
   * @property {Node[]} nodes - An array (arena) holding all node objects for this history.
   * @property {number | null} current - The index in the `nodes` array of the current state.
   * @property {number[]} redo_stack - A transient stack of indices for managing linear redo.
   */

  /**
   * @type {{fileName: string, content: string, history: History | null }}
   */
  let { fileName, content, history } = $derived(data);

  /** @type {ChunkItem[] | null} */
  let dirty_chunks = $state([]);

  onMount(() => {
    invoke('get_document_context', { name: fileName }).then((chunks) => {
      editorState.setGrammarChecks(chunks.data || []);
      dirty_chunks = chunks.data;
    });
  });

  let body = $derived(content || '');

  /**
   * @param {ChunkItem} item
   */
  function handleChunkPress(item) {
    const id = String(item.id.id.String);
    invoke('update_text_chunk', {
      id,
      correction: 'Please fix the grammar in this text.',
    }).then((result) => {
      console.log('Grammar check result:', result);
    });
  }
</script>

<div class="flex h-screen w-full bg-background">
  <aside
    class="border-r bg-card transition-all duration-300 ease-in-out h-dvh
    {appState.ui.isHistoryOpen ? 'w-60' : 'w-0'} overflow-hidden"
  >
    {#if appState.ui.isHistoryOpen}
      <div class="flex h-full flex-col">
        <div class="border-b px-4 py-3">
          <h2 class="font-semibold text-sm">History</h2>
        </div>
        <ScrollArea class="flex-1 h-full" type="scroll">
          {#if history}
            <EditorHistory {fileName} {history} />
          {:else}
            <div class="p-4 text-sm text-muted-foreground">No history available</div>
          {/if}
        </ScrollArea>
      </div>
    {/if}
  </aside>

  <div class="w-full h-full flex flex-1 flex-col">
    <main class="h-full flex-1 overflow-hidden">
      <ScrollArea class="flex-1 h-full" type="scroll">
        <a href="/">
          <Button variant="ghost" size="sm" class="mx-6 mt-6 text-neutral-500">
            <HomeIcon class="h-4 w-4" />
          </Button>
        </a>
        <div class="mx-auto max-w-4xl px-6 pb-8">
          <MarkdownEditor {fileName} {body} />
        </div>
      </ScrollArea>
    </main>

    <footer class="border-t bg-background">
      <EditorCommandbar {fileName} {body} currentVersion={history?.current || 0} />
    </footer>
  </div>

  <aside class="w-42 border-l bg-card overflow-hidden">
    <div class="flex h-full flex-col">
      <div class="border-b px-4 py-3">
        <h2 class="font-semibold text-sm">Outline</h2>
      </div>
      <ScrollArea class="flex-1 h-full" type="scroll">
        <ul class="space-y-2">
          {#each dirty_chunks as chunk}
            <li>
              <button onclick={() => handleChunkPress(chunk)} class="border p-1">
                {chunk.content}
              </button>
            </li>
          {/each}
        </ul>
        <EditorOutline {body} />
      </ScrollArea>
    </div>
  </aside>
</div>
