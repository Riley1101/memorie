<script>
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import MarkdownEditor from '$lib/components/md-editor.svelte';
  import EditorCommandbar from '$lib/components/editor-commandbar.svelte';
  import EditorHistory from '$lib/components/editor-history.svelte';
  import EditorOutline from '$lib/components/editor-outline.svelte';
  import Button from '$lib/components/ui/button/button.svelte';
  import HomeIcon from '@lucide/svelte/icons/home';
  import { appState } from '$lib/runes/app.svelte.js';

  let { data } = $props();

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
   * @type {{fileName: string, content: string, history: History | null}}
   */
  let { fileName, content, history } = $derived(data);

  let body = $derived(content || '');
</script>

<div class="flex h-screen w-full bg-background">
  <!-- Left Sidebar: History Panel -->
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

  <!-- Main Content Area -->
  <div class="w-full h-full flex flex-1 flex-col">
    <!-- Editor & Outline Container -->
    <!-- Center: Markdown Editor -->
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

    <!-- Right Sidebar: Outline/TOC -->
    <aside class="hidden w-64 border-l bg-card overflow-hidden">
      <div class="flex h-full flex-col">
        <div class="border-b px-4 py-3">
          <h2 class="font-semibold text-sm">Outline</h2>
        </div>
        <ScrollArea class="flex-1 h-full" type="scroll">
          <EditorOutline {body} />
        </ScrollArea>
      </div>
    </aside>

    <!-- Bottom: Command Bar -->
    <footer class="border-t bg-background">
      <EditorCommandbar {fileName} {body} currentVersion={history?.current || 0} />
    </footer>
  </div>
</div>
