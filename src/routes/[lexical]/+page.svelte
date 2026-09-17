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
  import EditorBacklinks from '$lib/components/editor-backlinks.svelte';
  import EditorSceneInfo from '$lib/components/editor-scene-info.svelte';
  import TableOfContentsIcon from '@lucide/svelte/icons/table-of-contents';
  import { MOD_KEY } from '$lib/keyboard.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { onMount } from 'svelte';
  import { editorState } from '$lib/runes/editor.svelte';
  import { syncGrammarChecks } from '$lib/hooks/editor-sync.svelte.js';
  import { goto, afterNavigate } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { formatTimeAgo, formatFileName } from '@/utils.js';
  import { fileManager, dirOf, baseOf } from '$lib/runes/fs.svelte';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { getCurrentWindow } from '@tauri-apps/api/window';

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
   * @type {{fileName: string, content: string, history: History | null, isDraft: boolean }}
   */
  let { fileName, content, history, isDraft } = $derived(data);

  // Bumps on real navigation between documents. A rename or a draft being
  // created moves the URL too, but flags itself so the editor stays mounted.
  let docKey = $state(0);
  afterNavigate((nav) => {
    if (nav.type === 'enter' || editorState.silentNavigation) return;
    docKey += 1;
    editorState.setSaveStatus({ status: 'idle', lastSaved: null });
  });

  // Ticks every 30s so "saved 2 minutes ago" stays honest.
  let now = $state(Date.now());

  onMount(() => {
    const timer = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(timer);
  });

  // Grammar chunks are only meaningful with local AI on; wait for config to arrive.
  $effect(() => {
    if (configManager.config?.ai_enabled && !isDraft) syncGrammarChecks(fileName);
  });

  let body = $derived(content || '');

  // Writings that share the same immediate parent folder as the current one:
  // a binder root, a chapter/scene folder, or the top-level content directory.
  let siblings = $derived.by(() => {
    const dir = dirOf(fileName);
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

  let saveLabel = $derived.by(() => {
    // `now` is read so the label recomputes on every tick.
    void now;
    const { status, lastSaved } = editorState.saveStatus;
    if (isDraft && status === 'idle') return 'New';
    switch (status) {
      case 'saving':
        return 'Saving…';
      case 'unsaved':
        return 'Unsaved changes';
      case 'error':
        return 'Save failed';
      case 'saved':
        return lastSaved ? `Saved ${formatTimeAgo(lastSaved)}` : 'Saved';
      default:
        return '';
    }
  });

  let saveTone = $derived(
    editorState.saveStatus.status === 'error'
      ? 'text-destructive'
      : editorState.saveStatus.status === 'unsaved'
        ? 'text-warning'
        : 'text-muted-foreground/70'
  );

  let binderLabel = $derived(dirOf(fileName));

  let focusMode = $derived(writingState.focusMode);

  /** Whether focus mode put the window into full screen (and so should undo it). */
  let enteredFullscreen = false;

  async function setFocusFullscreen(/** @type {boolean} */ on) {
    try {
      const win = getCurrentWindow();
      if (on) {
        if (await win.isFullscreen()) return;
        await win.setFullscreen(true);
        enteredFullscreen = true;
      } else if (enteredFullscreen) {
        enteredFullscreen = false;
        await win.setFullscreen(false);
      }
    } catch (e) {
      console.warn('Fullscreen unavailable:', e);
    }
  }

  // Focus mode takes the window full screen and gives it back afterwards,
  // including when the writer leaves the editor while still focused.
  $effect(() => {
    setFocusFullscreen(focusMode);
  });

  onMount(() => () => {
    writingState.toggleFocusMode(false);
    setFocusFullscreen(false);
  });
</script>

<div class="flex h-screen w-full bg-background" class:focus-mode={focusMode}>
  <aside
    class="transition-all duration-300 ease-in-out h-dvh bg-background/90 backdrop-blur-md
    {appState.ui.isHistoryOpen && !focusMode ? 'w-60 border-r border-border/40' : 'w-0'} overflow-hidden"
  >
    {#if appState.ui.isHistoryOpen && !focusMode}
      <div class="flex h-full flex-col pt-[var(--titlebar-height,0px)]">
        <div class="px-4 py-3">
          <h2 class="text-[0.6875rem] font-mono uppercase tracking-wider text-metadata">History</h2>
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

  <div class="w-full h-full flex flex-1 flex-col min-w-0">
    <header
      data-tauri-drag-region
      class="focus-chrome relative z-20 flex items-center command-bar__inner h-9 text-[0.6875rem] font-mono shrink-0
        {!appState.ui.isHistoryOpen ? 'pl-[max(var(--writer-padding-x),var(--titlebar-inset-left,0px))]' : ''}"
    >
      <div class="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-linear-to-r from-transparent via-border to-transparent"></div>
      <button
        onclick={() => goto(resolve('/'))}
        aria-label="Go home"
        title="Home"
        class="flex items-center justify-center size-7 -ml-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-foreground/5 transition-colors"
      >
        <HouseIcon strokeWidth={1.5} class="size-3.5" />
      </button>
      <div class="flex-1 flex items-center justify-center gap-1 min-w-0 pointer-events-none">
        {#if siblings.length > 1}
          <button
            onclick={() => goToWriting(prevWriting)}
            disabled={!prevWriting}
            aria-label="Previous writing"
            title={prevWriting ? formatFileName(baseOf(prevWriting.name)) : ''}
            class="pointer-events-auto flex items-center justify-center size-5 rounded text-muted-foreground/60 hover:text-foreground hover:bg-foreground/5 transition-colors disabled:opacity-0 disabled:pointer-events-none"
          >
            <ChevronLeftIcon strokeWidth={1.5} class="size-3" />
          </button>
        {/if}
        <span class="text-muted-foreground select-none truncate max-w-xs">
          {#if binderLabel}
            <span class="text-muted-foreground/50">{binderLabel.replaceAll('/', ' / ')} /</span>
          {/if}
          {editorState.name}
        </span>
        {#if siblings.length > 1}
          <button
            onclick={() => goToWriting(nextWriting)}
            disabled={!nextWriting}
            aria-label="Next writing"
            title={nextWriting ? formatFileName(baseOf(nextWriting.name)) : ''}
            class="pointer-events-auto flex items-center justify-center size-5 rounded text-muted-foreground/60 hover:text-foreground hover:bg-foreground/5 transition-colors disabled:opacity-0 disabled:pointer-events-none"
          >
            <ChevronRightIcon strokeWidth={1.5} class="size-3" />
          </button>
        {/if}
      </div>
      <span class="{saveTone} tracking-wide shrink-0 transition-colors" aria-live="polite">
        {saveLabel}
      </span>
      <button
        onclick={() => appState.toggleOutline()}
        aria-label={appState.ui.isOutlineOpen ? 'Hide table of contents' : 'Show table of contents'}
        aria-pressed={appState.ui.isOutlineOpen}
        title="Table of contents ({MOD_KEY}⇧O)"
        class="flex items-center justify-center size-7 ml-1.5 -mr-1.5 rounded-md transition-colors hover:text-foreground hover:bg-foreground/5
          {appState.ui.isOutlineOpen ? 'text-foreground' : 'text-muted-foreground'}"
      >
        <TableOfContentsIcon strokeWidth={1.5} class="size-3.5" />
      </button>
    </header>

    <main class="h-full flex-1 overflow-hidden">
      <ScrollFade class="h-full" fadeSize="h-12">
        <ScrollArea class="h-full" type="scroll">
          <div class="writing-surface pb-24">
            <MarkdownEditor {fileName} {body} {isDraft} {docKey} />
          </div>
        </ScrollArea>
      </ScrollFade>
    </main>

    <footer class="focus-chrome">
      <EditorCommandbar {fileName} {isDraft} currentVersion={history?.current || 0} />
    </footer>
  </div>

  <aside
    class="transition-all duration-300 ease-in-out h-dvh bg-background/90 backdrop-blur-md shrink-0
    {appState.ui.isOutlineOpen && !focusMode ? 'w-65 border-l border-border/40' : 'w-0'} overflow-hidden"
  >
    {#if appState.ui.isOutlineOpen && !focusMode}
      <div class="flex h-full flex-col pt-[var(--titlebar-height,0px)]">
        <ScrollFade class="flex-1 h-full">
          <ScrollArea class="h-full" type="scroll">
            <h2 class="px-4 py-3 text-[0.6875rem] font-mono uppercase tracking-wider text-metadata">Scene</h2>
            {#key fileName}
              <EditorSceneInfo {fileName} {isDraft} />
            {/key}
            <h2 class="px-4 pt-4 pb-3 text-[0.6875rem] font-mono uppercase tracking-wider text-metadata border-t border-border/40">Contents</h2>
            <EditorOutline />
            {#if !isDraft}
              <h2 class="px-4 pt-4 pb-3 text-[0.6875rem] font-mono uppercase tracking-wider text-metadata border-t border-border/40">
                Referenced by
              </h2>
              <EditorBacklinks {fileName} />
            {/if}
          </ScrollArea>
        </ScrollFade>
      </div>
    {/if}
  </aside>
</div>

<style>
  /* Focus mode: chrome fades out and comes back when the pointer reaches it
     or something inside it takes focus (command mode, the stats popover). */
  .focus-chrome {
    transition: opacity 300ms ease;
  }

  .focus-mode .focus-chrome {
    opacity: 0;
  }

  .focus-mode .focus-chrome:hover,
  .focus-mode .focus-chrome:focus-within {
    opacity: 1;
  }

  /* Room above the first line so typewriter scrolling can centre it too. */
  .focus-mode :global(.ProseMirror) {
    --writer-editor-pt: 40vh;
  }

  .focus-mode :global(.ProseMirror > *) {
    transition: opacity 200ms ease;
  }

  .focus-mode :global(.ProseMirror > :not(.is-current-block)) {
    opacity: 0.3;
  }
</style>
