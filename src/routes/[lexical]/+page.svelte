<script>
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import HouseIcon from '@lucide/svelte/icons/house';
  import MarkdownEditor from '$lib/components/md-editor.svelte';
  import EditorCommandbar from '$lib/components/editor-commandbar.svelte';
  import EditorHistory from '$lib/components/editor-history.svelte';
  import EditorOutline from '$lib/components/editor-outline.svelte';
  import Button from '$lib/components/ui/button/button.svelte';
  import { appState } from '$lib/runes/app.svelte.js';
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { syncGrammarChecks } from '$lib/hooks/editor-sync.svelte.js';
  import { goto } from '$app/navigation';

  let { data } = $props();

  /**
   * @typedef {Object} RecordPointer
   * @property {string} tb - The table name (e.g., 'chunk', 'documents').
   * @property {string} id - The record identifier wrapper.
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

  onMount(() => {
    syncGrammarChecks(fileName);
  });

  let body = $derived(content || '');

  /**
   * @param {ChunkItem} item
   */
  function handleChunkPress(item) {
    const id = String(item.id.id);
    invoke('update_text_chunk', {
      id,
      correction: 'Please fix the grammar in this text.',
    }).then(() => {
      syncGrammarChecks(fileName);
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
        <div class="writing-surface pb-24">
          <Button
            onclick={()=>goto('/')}
            variant="ghost"
            size="sm"
            disabled={false}
            class="-ml-1 mt-2 mb-4 text-neutral-500"
          >
            <HouseIcon class="size-4 opacity-20 hover:opacity-100 transition-opacity" />
          </Button>
          <MarkdownEditor {fileName} {body} />
        </div>
      </ScrollArea>
    </main>

    <footer class="bg-background">
      <EditorCommandbar {fileName} currentVersion={history?.current || 0} />
    </footer>
  </div>

  <aside class="hidden w-72 border-l bg-card overflow-hidden">
    <div class="flex h-full flex-col">
      <div class="border-b px-4 py-3">
        <h2 class="font-semibold text-sm">Outline</h2>
      </div>
      <ScrollArea class="flex-1 h-full" type="scroll">
        <ul class="space-y-2">
          {#each editorState.grammarChecks as chunk, index (index)}
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
