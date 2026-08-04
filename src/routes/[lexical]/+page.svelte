<script>
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import HouseIcon from '@lucide/svelte/icons/house';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import MarkdownEditor from '$lib/components/md-editor.svelte';
  import EditorCommandbar from '$lib/components/editor-commandbar.svelte';
  import EditorHistory from '$lib/components/editor-history.svelte';
  import EditorOutline from '$lib/components/editor-outline.svelte';
  import { appState } from '$lib/runes/app.svelte.js';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { editorState } from '$lib/runes/editor.svelte';
  import { syncGrammarChecks } from '$lib/hooks/editor-sync.svelte.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { formatTimeAgo, formatFileName } from '@/utils.js';
  import { fileManager } from '$lib/runes/fs.svelte';

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

  function dirOf(name) {
    const i = name.lastIndexOf('/');
    return i === -1 ? '' : name.slice(0, i);
  }

  // Writings that share the same immediate parent folder as the current one
  // (a binder root, or a chapter/scene folder nested inside a binder).
  let siblings = $derived.by(() => {
    const dir = dirOf(fileName);
    if (!dir) return [];
    return fileManager.files
      .filter((f) => dirOf(f.name) === dir)
      .sort((a, b) => a.name.localeCompare(b.name));
  });

  let siblingIndex = $derived(siblings.findIndex((f) => f.name === fileName));
  let prevWriting = $derived(siblingIndex > 0 ? siblings[siblingIndex - 1] : null);
  let nextWriting = $derived(
    siblingIndex !== -1 && siblingIndex < siblings.length - 1 ? siblings[siblingIndex + 1] : null
  );

  function goToWriting(file) {
    if (file) goto(resolve(`/${encodeURIComponent(file.name)}`));
  }

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
    class="transition-all duration-300 ease-in-out h-dvh bg-background/90 backdrop-blur-md
    {appState.ui.isHistoryOpen ? 'w-60' : 'w-0'} overflow-hidden"
  >
    {#if appState.ui.isHistoryOpen}
      <div class="flex h-full flex-col">
        <div class="px-4 py-3">
          <h2 class="font-semibold text-sm">History</h2>
        </div>
        <ScrollFade class="flex-1 h-full">
          <ScrollArea class="h-full" type="scroll">
            {#if history}
              <EditorHistory {fileName} {history} />
            {:else}
              <div class="p-4 text-sm text-muted-foreground">No history available</div>
            {/if}
          </ScrollArea>
        </ScrollFade>
      </div>
    {/if}
  </aside>

  <div class="w-full h-full flex flex-1 flex-col">
    <header class="relative flex items-center command-bar__inner h-9 text-[11px] font-mono shrink-0">
      <div class="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-linear-to-r from-transparent via-border to-transparent"></div>
      <button
        onclick={() => goto(resolve('/'))}
        class="flex items-center justify-center size-7 -ml-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-foreground/5 transition-colors"
      >
        <HouseIcon class="size-3.5" />
      </button>
      <div class="flex-1 flex items-center justify-center gap-1">
        {#if siblings.length > 1}
          <button
            onclick={() => goToWriting(prevWriting)}
            disabled={!prevWriting}
            title={prevWriting ? formatFileName(prevWriting.name.split('/').pop()) : ''}
            class="flex items-center justify-center size-5 rounded text-muted-foreground/60 hover:text-foreground hover:bg-foreground/5 transition-colors disabled:opacity-0 disabled:pointer-events-none"
          >
            <ChevronLeftIcon class="size-3" />
          </button>
        {/if}
        <span class="text-muted-foreground select-none truncate max-w-xs pointer-events-none">
          {editorState.name}
        </span>
        {#if siblings.length > 1}
          <button
            onclick={() => goToWriting(nextWriting)}
            disabled={!nextWriting}
            title={nextWriting ? formatFileName(nextWriting.name.split('/').pop()) : ''}
            class="flex items-center justify-center size-5 rounded text-muted-foreground/60 hover:text-foreground hover:bg-foreground/5 transition-colors disabled:opacity-0 disabled:pointer-events-none"
          >
            <ChevronRightIcon class="size-3" />
          </button>
        {/if}
      </div>
      <span class="text-muted-foreground/70 lowercase first-letter:uppercase tracking-wide">
        {editorState.saveStatus.status}
        {formatTimeAgo(editorState.saveStatus.lastSaved)}
      </span>
    </header>

    <main class="h-full flex-1 overflow-hidden">
      <ScrollFade class="h-full" fadeSize="h-12">
        <ScrollArea class="h-full" type="scroll">
          <div class="writing-surface pb-24">
            <MarkdownEditor {fileName} {body} />
          </div>
        </ScrollArea>
      </ScrollFade>
    </main>

    <footer>
      <EditorCommandbar {fileName} currentVersion={history?.current || 0} />
    </footer>
  </div>

  <aside class="hidden w-72 overflow-hidden bg-background/90 backdrop-blur-md">
    <div class="flex h-full flex-col">
      <div class="px-4 py-3">
        <h2 class="font-semibold text-sm">Outline</h2>
      </div>
      <ScrollFade class="flex-1 h-full">
        <ScrollArea class="h-full" type="scroll">
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
      </ScrollFade>
    </div>
  </aside>
</div>
