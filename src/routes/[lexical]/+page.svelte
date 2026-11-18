<script>
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import MarkdownEditor from '$lib/components/md-editor.svelte';
  import EditorCommandbar from '$lib/components/editor-commandbar.svelte';
  import EditorHistory from '$lib/components/editor-history.svelte';
  import EditorOutline from '$lib/components/editor-outline.svelte';
  import EditorBacklinks from '$lib/components/editor-backlinks.svelte';
  import EditorSceneInfo from '$lib/components/editor-scene-info.svelte';
  import EditorCodex from '$lib/components/editor-codex.svelte';
  import EditorFind from '$lib/components/editor-find.svelte';
  import ReviseBar from '$lib/components/revise-bar.svelte';
  import MarginNotes from '$lib/components/margin-notes.svelte';
  import EditorNotes from '$lib/components/editor-notes.svelte';
  import PanelLeftIcon from '@lucide/svelte/icons/panel-left';
  import PanelRightIcon from '@lucide/svelte/icons/panel-right';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import ShareIcon from '@lucide/svelte/icons/share';
  import BinderSidebar from '$lib/components/binder-sidebar.svelte';
  import EditorAssistant from '$lib/components/editor-assistant.svelte';
  import PanelResizer from '$lib/components/panel-resizer.svelte';
  import { IsMobile } from '$lib/hooks/is-mobile.svelte.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { shortcut, isMac } from '$lib/keyboard.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { onMount, untrack } from 'svelte';
  import { editorState } from '$lib/runes/editor.svelte';
  import { syncGrammarChecks } from '$lib/hooks/editor-sync.svelte.js';
  import { goto, afterNavigate } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { writingUrl } from '$lib/new-writing.js';
  import { formatTimeAgo, formatFileName } from '@/utils.js';
  import { fileManager, dirOf, baseOf } from '$lib/runes/fs.svelte';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { refreshSyncStatus } from '$lib/runes/sync.svelte.js';
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
    // Opening straight into a writing never mounts the home sidebar, so the
    // sync session would otherwise be unknown here.
    refreshSyncStatus().catch((e) => console.error('Could not read sync status:', e));
    return () => clearInterval(timer);
  });

  // Grammar chunks are only meaningful with local AI on; wait for config to arrive.
  $effect(() => {
    if (configManager.config?.ai_enabled && !isDraft) syncGrammarChecks(fileName);
  });

  // Remembered across launches so the home screen can offer the way back in.
  // A draft has nothing on disk to return to yet.
  $effect(() => {
    if (!isDraft && fileName) appState.setLastWriting(fileName);
  });

  // Deleted from somewhere else (the palette, another window, the trash being
  // emptied): leaving the writer on a path that no longer exists means every
  // later save fails. Step back home instead.
  $effect(() => {
    if (isDraft || !fileManager.hasLoadedFiles) return;
    const stillThere = fileManager.files.some((f) => f.name === fileName);
    if (stillThere) return;
    untrack(() => {
      appState.setLastWriting(null);
      goto(resolve('/'));
    });
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
    if (file) goto(writingUrl(file.name));
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
        // Normal while typing; autosave is a beat behind. Say so quietly.
        return 'Editing…';
      case 'error':
        return 'Couldn’t save · Try again';
      case 'saved':
        return lastSaved ? `Saved ${formatTimeAgo(lastSaved)}` : 'Saved';
      default:
        return '';
    }
  });

  // Colour is kept for the one state that needs the writer: a failed save.
  let saveTone = $derived(
    editorState.saveStatus.status === 'error' ? 'text-destructive' : 'text-muted-foreground'
  );

  /** The library sidebar, opened over the page from the toolbar. */
  let librarySheetOpen = $state(false);

  /**
   * Below the docked breakpoint there is no room for the library beside the
   * writing, so the toolbar button opens it as a sheet instead of docking it.
   */
  const narrow = new IsMobile(1024);

  const SCENE_MIN = 240;
  const SCENE_MAX = 720;
  /** Untouched, the panel takes this share of the window. */
  const SCENE_SHARE = 0.3;

  /** The scene panel's width, dragged on its inner edge and remembered. */
  let sceneWidth = $state(readSceneWidth());

  function readSceneWidth() {
    try {
      const saved = Number(localStorage.getItem('scenePanelWidth'));
      if (Number.isFinite(saved) && saved > 0) {
        return Math.min(SCENE_MAX, Math.max(SCENE_MIN, saved));
      }
    } catch {
      // Not remembered; the share below stands.
    }
    if (typeof window === 'undefined') return 360;
    return Math.round(Math.min(SCENE_MAX, Math.max(SCENE_MIN, window.innerWidth * SCENE_SHARE)));
  }

  /** @param {number} next */
  function setSceneWidth(next) {
    sceneWidth = next;
    try {
      localStorage.setItem('scenePanelWidth', String(next));
    } catch {
      // Not remembered; fine.
    }
  }

  let focusMode = $derived(writingState.focusMode);

  /**
   * True while the library panel is a column on the left. It then covers the
   * traffic lights with its own drag strip, as on the library screens; with it
   * away the toolbar has to leave room for them instead.
   */
  let libraryDocked = $derived(appState.ui.isLibraryOpen && !focusMode && !narrow.current);

  const onMacOS = isMac();

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

  // Focus mode takes the window full screen only if the writer asked it to,
  // and gives it back afterwards — including when they leave the editor while
  // still focused.
  $effect(() => {
    setFocusFullscreen(focusMode && writingState.focusFullscreen);
  });

  onMount(() => () => {
    writingState.toggleFocusMode(false);
    setFocusFullscreen(false);
  });
</script>

{#snippet iconButton(
  /** @type {{ label: string, tip: string, onclick: () => void, pressed?: boolean, disabled?: boolean, icon: any }} */ b
)}
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <button
          {...props}
          type="button"
          class="tb-btn"
          aria-label={b.label}
          aria-pressed={b.pressed}
          disabled={b.disabled}
          onclick={b.onclick}
        >
          <b.icon strokeWidth={1.7} />
        </button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="bottom" portalProps={{}}>{b.tip}</Tooltip.Content>
  </Tooltip.Root>
{/snippet}

<!-- The same frame as the library screens: panels and paper laid on the desk
     with one gutter, and the toolbar inside the paper rather than across the
     window. On macOS the library panel takes the traffic lights in its top
     strip; with that panel away, the toolbar indents to leave them room. -->
<div class="flex h-full w-full gap-3 p-3 bg-frame overflow-hidden" class:focus-mode={focusMode}>
  <BinderSidebar
    docked={appState.ui.isLibraryOpen && !focusMode}
    bind:activeBinder={appState.ui.activeBinder}
    bind:sheetOpen={librarySheetOpen}
  />

  {#if appState.ui.isHistoryOpen && !focusMode}
    <aside
      aria-label="History"
      class="well flex flex-col w-66 shrink-0 rounded-group overflow-hidden"
    >
      {#if onMacOS && !libraryDocked}
        <!-- Leftmost panel with the library away: the traffic lights land here,
             so it keeps the same empty strip the library panel does. -->
        <div data-tauri-drag-region="deep" class="h-[var(--titlebar-strip,14px)] shrink-0"></div>
      {/if}
      <div class="flex items-center justify-between gap-3 px-5 pt-4 pb-3">
        <h2 class="text-xs font-semibold text-muted-foreground">History</h2>
        <span class="meta truncate" title={baseOf(fileName)}
          >{formatFileName(baseOf(fileName))}</span
        >
      </div>
      <ScrollFade class="flex-1 min-h-0">
        <ScrollArea class="h-full" type="scroll">
          {#if history}
            <EditorHistory {fileName} {history} />
          {:else}
            <div class="p-4 text-sm text-muted-foreground">No history yet.</div>
          {/if}
        </ScrollArea>
      </ScrollFade>
    </aside>
  {/if}

  <main class="lifted relative h-full flex-1 min-w-0 flex flex-col overflow-hidden rounded-group">
    <!-- The paper's own toolbar: the library on the left, the writing's title in
         the middle, its panels and export on the right. -->
    <header
      data-tauri-drag-region="deep"
      class="focus-chrome toolbar relative z-20 gap-3 {libraryDocked
        ? ''
        : 'pl-[max(20px,var(--titlebar-inset-left,0px))]'}"
    >
      <div class="capsule glass">
        {@render iconButton({
          label: 'Show library sidebar',
          tip: 'Library',
          onclick: () => (narrow.current ? (librarySheetOpen = true) : appState.toggleLibrary()),
          icon: PanelLeftIcon,
        })}
      </div>

      <div class="flex-1 min-w-0 flex items-center justify-center gap-1 pointer-events-none">
        {#if siblings.length > 1}
          <span class="pointer-events-auto">
            {@render iconButton({
              label: 'Previous writing',
              tip: prevWriting
                ? `Previous writing: ${formatFileName(baseOf(prevWriting.name))}`
                : 'Previous writing',
              onclick: () => goToWriting(prevWriting),
              disabled: !prevWriting,
              icon: ChevronLeftIcon,
            })}
          </span>
        {/if}
        <div class="min-w-0 flex flex-col items-center leading-tight select-none">
          <h1 class="m-0 max-w-[40ch] truncate text-sm font-semibold text-foreground">
            {editorState.name || 'Untitled'}
          </h1>
        </div>
        {#if siblings.length > 1}
          <span class="pointer-events-auto">
            {@render iconButton({
              label: 'Next writing',
              tip: nextWriting
                ? `Next writing: ${formatFileName(baseOf(nextWriting.name))}`
                : 'Next writing',
              onclick: () => goToWriting(nextWriting),
              disabled: !nextWriting,
              icon: ChevronRightIcon,
            })}
          </span>
        {/if}
      </div>

      {#if saveLabel}
        <span class="meta max-w-[28ch] truncate" aria-live="polite">
          {#if editorState.saveStatus.status === 'error'}
            <button
              type="button"
              onclick={() => editorState.flushSave?.()}
              class="{saveTone} hover:underline underline-offset-2"
            >
              {saveLabel}
            </button>
          {:else}
            <span class="{saveTone} transition-colors">{saveLabel}</span>
          {/if}
        </span>
      {/if}

      <div class="capsule glass">
        {@render iconButton({
          label: appState.ui.isHistoryOpen ? 'Hide history' : 'Show history',
          tip: `History · ${shortcut('U')}`,
          onclick: () => appState.toggleChatHistory(),
          pressed: appState.ui.isHistoryOpen,
          disabled: isDraft,
          icon: HistoryIcon,
        })}
        {@render iconButton({
          label: appState.ui.isOutlineOpen ? 'Hide scene panel' : 'Show scene panel',
          tip: `Scene panel · ${shortcut('shift', 'O')}`,
          onclick: () => appState.toggleOutline(),
          pressed: appState.ui.isOutlineOpen,
          icon: PanelRightIcon,
        })}
      </div>
      <div class="capsule glass">
        {@render iconButton({
          label: 'Export',
          tip: 'Export…',
          onclick: () => appState.openExport({ fileName, dir: dirOf(fileName) }),
          disabled: isDraft,
          icon: ShareIcon,
        })}
      </div>
    </header>

    <!-- The writing itself. Its own positioning context, so the find bar, the
         revise bar and the command bar sit inside the paper, clear of the
         toolbar above them. -->
    <div class="relative flex-1 min-h-0">
      <EditorFind />
      <ReviseBar />
      <ScrollFade class="h-full" fade={24}>
        <ScrollArea class="h-full" type="scroll">
          <div class="writing-surface relative pb-24">
            <MarkdownEditor {fileName} {body} {isDraft} {docKey} />
            <MarginNotes />
          </div>
        </ScrollArea>
      </ScrollFade>
      <footer class="focus-chrome">
        <EditorCommandbar {fileName} {isDraft} />
      </footer>
    </div>
  </main>

  {#if appState.ui.isOutlineOpen && !focusMode}
    <!-- The scene panel: an inspector floating at the window's right edge. -->
    <aside
      aria-label="Scene"
      class="well relative flex flex-col shrink-0 rounded-group overflow-hidden"
      style="width: {sceneWidth}px"
    >
      <PanelResizer
        width={sceneWidth}
        min={SCENE_MIN}
        max={SCENE_MAX}
        label="Resize the scene panel"
        onresize={setSceneWidth}
      />
      <ScrollFade class="flex-1 min-h-0">
        <ScrollArea class="h-full" type="scroll">
          <div class="pb-3">
            <h2 class="inspector-head">Scene</h2>
            {#key fileName}
              <EditorSceneInfo {fileName} {isDraft} />
            {/key}
            <h2 class="inspector-head">Notes</h2>
            <EditorNotes />
            <h2 class="inspector-head">In this scene</h2>
            <EditorCodex {fileName} content={body} />
            <h2 class="inspector-head">Contents</h2>
            <EditorOutline />
            {#if !isDraft}
              <h2 class="inspector-head">Referenced by</h2>
              <EditorBacklinks {fileName} />
            {/if}
          </div>
        </ScrollArea>
      </ScrollFade>
    </aside>
  {/if}

  {#if configManager.config?.ai_enabled && appState.ui.isChatOpen}
    <EditorAssistant />
  {/if}
</div>

<style>
  .inspector-head {
    padding: 14px 14px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted-foreground);
  }

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
