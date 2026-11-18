<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import BinderSidebar from '$lib/components/binder-sidebar.svelte';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { shortcut } from '$lib/keyboard.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { startNewWriting } from '$lib/new-writing.js';
  import { fileManager, baseOf, dirOf } from '$lib/runes/fs.svelte.js';
  import { formatFileName, formatTimeAgo } from '$lib/utils.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { binderDot } from '$lib/binder-tints.js';
  import ShareIcon from '@lucide/svelte/icons/share';
  import ListIcon from '@lucide/svelte/icons/list';
  import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
  import PanelLeftIcon from '@lucide/svelte/icons/panel-left';

  let newLabel = $derived(
    appState.ui.activeBinder ? `New in ${appState.ui.activeBinder}` : 'New writing'
  );

  /**
   * The writing to offer a way back into. Only shown once the listing is in
   * and the file is still there, so a deleted or renamed writing never leaves
   * a dead button on the home screen.
   */
  let resumable = $derived.by(() => {
    const last = appState.ui.lastWriting;
    if (!last || !fileManager.hasLoadedFiles) return null;
    return fileManager.files.find((f) => f.name === last) ?? null;
  });

  let resumeLabel = $derived(resumable ? formatFileName(baseOf(resumable.name)) : '');
  let resumeBinder = $derived(resumable ? dirOf(resumable.name).split('/')[0] : '');

  function resume() {
    if (resumable) goto(resolve(`/${encodeURIComponent(resumable.name)}`));
  }

  /** List or cards; one choice for the whole library, remembered across launches. */
  let view = $state(/** @type {'list' | 'cards'} */ (readView()));

  function readView() {
    try {
      const saved = localStorage.getItem('homeView') ?? localStorage.getItem('binderView');
      return saved === 'cards' ? 'cards' : 'list';
    } catch {
      return 'list';
    }
  }

  /** @param {'list' | 'cards'} next */
  function setView(next) {
    view = next;
    try {
      localStorage.setItem('homeView', next);
    } catch {
      // Not remembered; fine.
    }
  }

  let sidebarOpen = $state(false);
</script>

{#snippet continueStrip()}
  <!-- Continue writing (C3): one slim strip back into the last writing. -->
  {#if resumable}
    <a
      href={resolve(`/${encodeURIComponent(resumable.name)}`)}
      onclick={(e) => {
        if (e.metaKey || e.ctrlKey || e.shiftKey) return;
        e.preventDefault();
        resume();
      }}
      class="glass mt-2 flex items-center gap-3 rounded-group py-2 pr-2 pl-4 text-foreground hover:bg-selection transition-colors"
    >
      <span class="binder-dot {binderDot(resumeBinder)}"></span>
      <span class="text-xs font-semibold text-primary dark:text-[color-mix(in_oklch,var(--primary)_55%,white)]">Continue</span>
      <span class="font-serif text-base whitespace-nowrap truncate max-w-[40%]">{resumeLabel}</span>
      <span class="meta flex-1 min-w-0 truncate">
        {[resumeBinder, `edited ${formatTimeAgo(resumable.last_modified * 1000)}`].filter(Boolean).join(' · ')}
      </span>
      <span class="tb-btn tb-primary px-3.5 shrink-0">Resume</span>
    </a>
  {/if}
{/snippet}

<div class="relative flex w-full h-full overflow-hidden">
  <BinderSidebar bind:activeBinder={appState.ui.activeBinder} bind:sheetOpen={sidebarOpen} />

  <main class="flex-1 min-w-0 h-full flex flex-col bg-paper shadow-[-1px_0_0_var(--hair)]">
    <header
      data-tauri-drag-region
      class="toolbar max-lg:pl-[max(20px,var(--titlebar-inset-left,0px))]"
    >
      <button
        type="button"
        class="tb-btn lg:hidden -ml-2"
        aria-label="Show library sidebar"
        onclick={() => (sidebarOpen = true)}
      >
        <PanelLeftIcon strokeWidth={1.7} />
      </button>
      <h1 class="toolbar-title" data-tauri-drag-region>{appState.ui.activeBinder ?? 'All writings'}</h1>
      <div class="ml-auto flex items-center gap-2">
        <div class="capsule glass" role="group" aria-label="View">
          <button type="button" class="tb-btn" aria-label={appState.ui.activeBinder ? 'Chapters' : 'List'} aria-pressed={view === 'list'} onclick={() => setView('list')}>
            <ListIcon strokeWidth={1.7} />
          </button>
          <button type="button" class="tb-btn" aria-label="Cards" aria-pressed={view === 'cards'} onclick={() => setView('cards')}>
            <LayoutGridIcon strokeWidth={1.7} />
          </button>
        </div>

        <!-- Export is a headline feature; it was reachable only from the palette. -->
        {#if appState.ui.activeBinder}
          <div class="capsule glass">
            <Tooltip.Root>
              <Tooltip.Trigger>
                {#snippet child({ props })}
                  <button
                    {...props}
                    type="button"
                    class="tb-btn"
                    onclick={() => appState.openExport({ fileName: null, dir: appState.ui.activeBinder ?? '' })}
                    aria-label="Export {appState.ui.activeBinder}"
                  >
                    <ShareIcon strokeWidth={1.7} />
                  </button>
                {/snippet}
              </Tooltip.Trigger>
              <Tooltip.Content side="bottom" portalProps={{}}>Export {appState.ui.activeBinder}…</Tooltip.Content>
            </Tooltip.Root>
          </div>
        {/if}

        <!-- Primary action: one click, no naming, lands in the open binder. -->
        <div class="capsule tb-primary p-0">
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={() => startNewWriting(appState.ui.activeBinder ?? '')}
                  class="tb-btn pl-3 pr-2"
                >
                  <PlusIcon strokeWidth={2} class="size-[15px]!" />
                  <span class="hidden sm:inline max-w-[22ch] truncate">{newLabel}</span>
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="bottom" portalProps={{}}>{newLabel} · {shortcut('N')}</Tooltip.Content>
          </Tooltip.Root>
          <span aria-hidden="true" class="w-px h-4 bg-primary-foreground/30"></span>
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={() => appState.toggleNewPicker(true)}
                  aria-label="New writing in another binder"
                  class="tb-btn min-w-6.5 pl-1.5 pr-2"
                >
                  <ChevronDownIcon strokeWidth={2} class="size-[13px]!" />
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="bottom" portalProps={{}}>New in… · {shortcut('shift', 'N')}</Tooltip.Content>
          </Tooltip.Root>
        </div>
      </div>
    </header>

    <div class="flex-1 min-h-0 w-full px-5">
      <div class="mx-auto h-full max-w-215">
        <TimelineDocuments activeBinder={appState.ui.activeBinder} {view} header={continueStrip} />
      </div>
    </div>
  </main>
</div>
