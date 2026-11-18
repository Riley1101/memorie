<script>
  import { onMount, tick } from 'svelte';
  import { fileManager } from '@/runes/fs.svelte';
  import { treeState } from '@/runes/tree.svelte.js';
  import { naturalCollator } from '@/utils';
  import CornerDownRightIcon from '@lucide/svelte/icons/corner-down-right';
  import { syncTarget, refreshSyncStatus, publish } from '@/runes/sync.svelte.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import SearchIcon from '@lucide/svelte/icons/search';
  import SettingIcon from '@lucide/svelte/icons/settings';
  import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import ScrollFade from './scroll-fade.svelte';
  import { appState } from '$lib/runes/app.svelte.js';
  import { shortcut, isMac } from '$lib/keyboard.svelte.js';
  import { page } from '$app/state';
  import ListIcon from '@lucide/svelte/icons/list';
  import CloudIcon from '@lucide/svelte/icons/cloud';
  import { openCodex } from '$lib/runes/codex.svelte.js';
  import { offerUndo } from '$lib/runes/trash.svelte.js';
  import { BINDER_COLOURS, BINDER_COLOUR_NAMES } from '$lib/runes/app.svelte.js';
  import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import PaletteIcon from '@lucide/svelte/icons/palette';

  /**
   * The library sidebar: a glass panel floating on the window frame.
   * `docked` shows it as a column from 1024px up (the library screens); below
   * that, and always where `docked` is false (the editor), it opens as a sheet
   * from the toolbar's sidebar button, bound through `sheetOpen`.
   */
  let { activeBinder = $bindable(null), docked = true, sheetOpen = $bindable(false) } = $props();

  let isCreatingBinder = $state(false);
  let newBinderName = $state('');

  const onMacOS = isMac();

  /** Binders and "All writings" filter the library, so they lead home from anywhere else. */
  function showInLibrary(/** @type {string | null} */ name) {
    activeBinder = name;
    if (page.url.pathname !== '/') goto(resolve('/'));
  }

  function selectBinder(name) {
    if (page.url.pathname === '/' && activeBinder === name) activeBinder = null;
    else showInLibrary(name);
  }

  let onCodex = $derived(page.url.pathname.startsWith('/codex'));
  let onHome = $derived(page.url.pathname === '/');

  function binderCount(name) {
    return fileManager.files.filter((f) => f.folder === name).length;
  }

  /** Nested folders of the open binder, in tree order, for the jump list. */
  let binderFolders = $derived.by(() => {
    if (!activeBinder) return [];
    const prefix = `${activeBinder}/`;
    return fileManager.folders
      .filter((f) => f.startsWith(prefix))
      .sort((a, b) => naturalCollator.compare(a, b))
      .map((path) => {
        const segments = path.slice(prefix.length).split('/');
        return { path, label: segments[segments.length - 1], depth: segments.length };
      });
  });

  function jumpToFolder(path) {
    treeState.reveal(path);
    tick().then(() => {
      const row = document.querySelector(`[data-tree-folder="${CSS.escape(path)}"]`);
      row?.scrollIntoView({ block: 'center', behavior: 'smooth' });
      /** @type {HTMLElement | null} */ (row?.querySelector('[data-tree-row]'))?.focus({ preventScroll: true });
    });
  }

  function goToManageBinders() {
    sheetOpen = false;
    goto(resolve('/binders'));
  }

  function startCreateBinder() {
    newBinderName = '';
    isCreatingBinder = true;
  }

  function cancelCreateBinder() {
    isCreatingBinder = false;
    newBinderName = '';
  }

  let isCreating = false;

  async function handleCreateBinder() {
    if (isCreating) return;
    if (!newBinderName.trim()) {
      cancelCreateBinder();
      return;
    }
    const name = newBinderName.trim();
    isCreating = true;
    newBinderName = '';
    isCreatingBinder = false;
    try {
      await fileManager.createBinder(name);
      if (fileManager.binders.includes(name)) activeBinder = name;
    } finally {
      isCreating = false;
    }
  }

  /** Blur commits a typed name; an empty box just closes. */
  function handleCreateBlur() {
    if (newBinderName.trim()) handleCreateBinder();
    else cancelCreateBinder();
  }

  /** Publish goes to the preferred storage only, matching `:sync` and auto-push on exit. */
  const target = $derived(syncTarget());

  onMount(() => {
    refreshSyncStatus();
  });

  function openSearch(onSelect) {
    onSelect?.();
    appState.toggleCommandMenu(true);
  }

  function openSettings(onSelect) {
    onSelect?.();
    goto(resolve('/settings'));
  }

  /** "Work" is always teal (spec); other binders share the clay accent. */
  /**
   * Tailwind needs these class names written out, so the colours live in a
   * table rather than being built from the colour name. The active label stays
   * in full ink: the tint colours as text fall well under 4.5:1 on the page.
   * @type {Record<string, { icon: string, swatch: string, activeBg: string, activeText: string }>}
   */
  const TINTS = {
    teal: { icon: 'text-binder-teal', swatch: 'bg-binder-teal', activeBg: 'bg-binder-teal/10', activeText: 'text-foreground' },
    primary: { icon: 'text-binder-clay', swatch: 'bg-binder-clay', activeBg: 'bg-binder-clay/10', activeText: 'text-foreground' },
    info: { icon: 'text-binder-slate', swatch: 'bg-binder-slate', activeBg: 'bg-binder-slate/10', activeText: 'text-foreground' },
    success: { icon: 'text-binder-sage', swatch: 'bg-binder-sage', activeBg: 'bg-binder-sage/10', activeText: 'text-foreground' },
    warning: { icon: 'text-binder-ochre', swatch: 'bg-binder-ochre', activeBg: 'bg-binder-ochre/10', activeText: 'text-foreground' },
    accent: { icon: 'text-binder-plum', swatch: 'bg-binder-plum', activeBg: 'bg-binder-plum/10', activeText: 'text-foreground' },
    muted: {
      icon: 'text-muted-foreground',
      swatch: 'bg-muted-foreground/50',
      activeBg: 'bg-muted',
      activeText: 'text-foreground',
    },
  };

  /** @param {string | null} name */
  function binderTint(name) {
    return TINTS[appState.binderColour(name)] ?? TINTS.primary;
  }

  // --- Renaming, recolouring and deleting a binder, without leaving here ---

  let renamingBinder = $state(/** @type {string | null} */ (null));
  let binderRenameValue = $state('');
  let binderToDelete = $state(/** @type {string | null} */ (null));

  /** @param {string} name */
  function startRenameBinder(name) {
    renamingBinder = name;
    binderRenameValue = name;
  }

  function cancelRenameBinder() {
    renamingBinder = null;
    binderRenameValue = '';
  }

  async function confirmRenameBinder() {
    const from = renamingBinder;
    const to = binderRenameValue.trim();
    cancelRenameBinder();
    if (!from || !to || to === from) return;
    await fileManager.renameBinder(from, to);
    // The colour is keyed by name, so it has to follow the rename.
    const colour = appState.binderColours[from];
    if (colour) {
      appState.setBinderColour(to, colour);
      appState.setBinderColour(from, null);
    }
    if (activeBinder === from) activeBinder = to;
  }

  /** Writings anywhere under a binder, not just directly inside it. */
  function binderTotal(name) {
    return fileManager.filesUnder(name).length;
  }

  async function confirmDeleteBinder() {
    const name = binderToDelete;
    binderToDelete = null;
    if (!name) return;
    const entry = await fileManager.deleteFolderRecursive(name);
    if (entry) offerUndo(entry);
    if (activeBinder === name) activeBinder = null;
  }
</script>

{#snippet searchField(onSelect)}
  <button type="button" onclick={() => openSearch(onSelect)} class="field w-full text-left hover:text-foreground transition-colors">
    <SearchIcon strokeWidth={1.8} class="size-3.5 shrink-0" />
    <span class="flex-1 truncate">Search</span>
    <kbd class="font-sans text-xs text-muted-foreground shrink-0">{shortcut('K')}</kbd>
  </button>
{/snippet}

{#snippet binderPanel(onSelect)}
  <ScrollFade class="min-h-0 flex-1" fade={16}>
    <ScrollArea type="scroll" class="h-full">
      <nav aria-label="Library" class="flex flex-col pb-2">
        <div class="sb-head">Library</div>
        <button
          type="button"
          class="sb-row"
          aria-current={onHome && activeBinder === null ? 'page' : undefined}
          onclick={() => {
            showInLibrary(null);
            onSelect?.();
          }}
        >
          <ListIcon strokeWidth={1.7} />
          <span class="truncate flex-1">All writings</span>
          <span class="sb-count">{fileManager.files.length}</span>
        </button>
        <button
          type="button"
          class="sb-row"
          aria-current={onCodex ? 'page' : undefined}
          onclick={() => {
            onSelect?.();
            openCodex(activeBinder ? { binder: activeBinder } : {});
          }}
        >
          <BookUserIcon strokeWidth={1.7} />
          <span class="truncate flex-1">Codex</span>
        </button>

        <div class="sb-head">
          <button type="button" onclick={goToManageBinders} class="hover:text-foreground transition-colors" title="Manage binders">
            Binders
          </button>
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={startCreateBinder}
                  aria-label="New binder"
                  class="ml-auto flex items-center justify-center size-5 rounded-[5px] text-muted-foreground hover:text-foreground hover:bg-fill transition-colors"
                >
                  <PlusIcon class="size-3.5" strokeWidth={1.8} />
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="right" portalProps={{}}>New binder</Tooltip.Content>
          </Tooltip.Root>
        </div>

        {#if isCreatingBinder}
          <div class="sb-row bg-fill">
            <span class="binder-dot bg-muted-foreground/40"></span>
            <!-- These fields only appear because the writer asked to type in one (rename, new folder, new binder), so focus belongs here. -->
            <!-- svelte-ignore a11y_autofocus -->
            <input
              bind:value={newBinderName}
              type="text"
              placeholder="Binder name"
              aria-label="New binder name"
              autofocus
              onkeydown={(e) => {
                if (e.key === 'Enter') handleCreateBinder();
                if (e.key === 'Escape') cancelCreateBinder();
              }}
              onblur={handleCreateBlur}
              class="flex-1 min-w-0 bg-transparent outline-none placeholder:text-placeholder"
            />
          </div>
        {/if}

        {#each fileManager.binders as binder (binder)}
          {@const tint = binderTint(binder)}
          {#if renamingBinder === binder}
            <div class="sb-row bg-fill">
              <span class="binder-dot {tint.swatch}"></span>
              <!-- svelte-ignore a11y_autofocus -->
              <input
                bind:value={binderRenameValue}
                type="text"
                aria-label="Binder name"
                autofocus
                onkeydown={(e) => {
                  if (e.key === 'Enter') confirmRenameBinder();
                  if (e.key === 'Escape') cancelRenameBinder();
                }}
                onblur={confirmRenameBinder}
                class="flex-1 min-w-0 bg-transparent outline-none"
              />
            </div>
          {:else}
            <!-- Renaming, recolouring and deleting a binder, right where binders are chosen. -->
            <ContextMenu.Root>
              <ContextMenu.Trigger class="">
                <button
                  type="button"
                  class="sb-row"
                  aria-current={onHome && activeBinder === binder ? 'page' : undefined}
                  onclick={() => {
                    selectBinder(binder);
                    onSelect?.();
                  }}
                >
                  <span class="binder-dot {tint.swatch}"></span>
                  <span class="truncate flex-1">{binder}</span>
                  <span class="sb-count">{binderCount(binder)}</span>
                </button>
              </ContextMenu.Trigger>
              <ContextMenu.Content class="w-48">
                <ContextMenu.Item class="" inset={false} onclick={() => startRenameBinder(binder)}>
                  <PencilIcon class="size-3.5" strokeWidth={1.5} />
                  Rename
                </ContextMenu.Item>
                <ContextMenu.Sub>
                  <ContextMenu.SubTrigger class="" inset={false}>
                    <PaletteIcon class="size-3.5" strokeWidth={1.5} />
                    Colour
                  </ContextMenu.SubTrigger>
                  <ContextMenu.SubContent class="w-40">
                    {#each BINDER_COLOURS as colour (colour)}
                      <ContextMenu.Item class="" inset={false} onclick={() => appState.setBinderColour(binder, colour)}>
                        <span class="binder-dot {TINTS[colour].swatch}"></span>
                        <span class="flex-1">{BINDER_COLOUR_NAMES[colour]}</span>
                      </ContextMenu.Item>
                    {/each}
                    <ContextMenu.Separator class="" />
                    <ContextMenu.Item class="" inset={false} onclick={() => appState.setBinderColour(binder, null)}>
                      From the name
                    </ContextMenu.Item>
                  </ContextMenu.SubContent>
                </ContextMenu.Sub>
                <ContextMenu.Separator class="" />
                <ContextMenu.Item class="" inset={false} variant="destructive" onclick={() => (binderToDelete = binder)}>
                  <Trash2Icon class="size-3.5" strokeWidth={1.5} />
                  Delete binder
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Root>
          {/if}
          {#if onHome && activeBinder === binder && binderFolders.length}
            <!-- Chapters and scenes of the open binder: click jumps the tree to that folder. -->
            {#each binderFolders as folder (folder.path)}
              <button
                type="button"
                onclick={() => {
                  jumpToFolder(folder.path);
                  onSelect?.();
                }}
                title={folder.path}
                class="sb-row h-6 text-xs text-muted-foreground hover:text-foreground"
                style="padding-left: {10 + folder.depth * 12}px"
              >
                <CornerDownRightIcon strokeWidth={1.5} class="size-3 shrink-0 text-muted-foreground/50" />
                <span class="truncate flex-1">{folder.label}</span>
              </button>
            {/each}
          {/if}
        {/each}
      </nav>
    </ScrollArea>
  </ScrollFade>

  <!-- Bottom of the panel: where a deleted writing is looked for, whether the
       work has left the machine, and the way into settings. -->
  <div class="flex flex-col gap-px pt-1 shrink-0">
    <button
      type="button"
      class="sb-row"
      onclick={() => {
        onSelect?.();
        appState.toggleTrash(true);
      }}
    >
      <Trash2Icon strokeWidth={1.7} />
      <span class="truncate flex-1">Recently deleted</span>
    </button>
    {#if target.provider !== 'none'}
      {#if target.ready && target.pending > 0}
        <button type="button" class="sb-row" onclick={publish} disabled={target.busy} title="Publish to {target.label}">
          <UploadCloudIcon strokeWidth={1.7} />
          <span class="truncate flex-1">{target.busy ? 'Publishing…' : 'Publish'}</span>
          <span class="sb-count">{target.pending}</span>
        </button>
      {:else}
        <div class="sb-row hover:bg-transparent" role="status">
          <CloudIcon strokeWidth={1.7} />
          <span class="truncate flex-1">{target.ready ? `Synced to ${target.label}` : `${target.label} not set up`}</span>
        </div>
      {/if}
    {/if}
    <button
      type="button"
      class="sb-row"
      aria-current={page.url.pathname.startsWith('/settings') ? 'page' : undefined}
      onclick={() => openSettings(onSelect)}
    >
      <SettingIcon strokeWidth={1.7} />
      <span class="truncate flex-1">Settings</span>
    </button>
  </div>
{/snippet}

{#snippet panel(onSelect)}
  {#if onMacOS}
    <!-- The traffic lights sit here; the strip moves the window. -->
    <div data-tauri-drag-region class="h-9 shrink-0 -mt-2 -mx-2 mb-1"></div>
  {/if}
  {@render searchField(onSelect)}
  {@render binderPanel(onSelect)}
{/snippet}

{#if docked}
  <!-- From 1024px the library sidebar is a glass panel floating on the frame. -->
  <aside
    aria-label="Library"
    class="glass hidden lg:flex flex-col w-59 shrink-0 m-2 mr-0 p-2 rounded-group min-h-0 h-[calc(100%-16px)]"
  >
    {@render panel()}
  </aside>
{/if}

<Sheet.Root bind:open={sheetOpen}>
  <Sheet.Content side="left" class="w-64 p-2 gap-0 flex flex-col" portalProps={{}}>
    <Sheet.Title class="sr-only">Library</Sheet.Title>
    {#if onMacOS}
      <div class="h-9 shrink-0 -mt-2"></div>
    {/if}
    <div class="pr-8 pb-1">
      {@render searchField(() => (sheetOpen = false))}
    </div>
    {@render binderPanel(() => (sheetOpen = false))}
  </Sheet.Content>
</Sheet.Root>

<!-- Deleting a binder takes everything in it, so it asks first and says how
     much — then sends the lot to the trash rather than off the disk. -->
<Dialog.Root open={binderToDelete !== null} onOpenChange={(v) => { if (!v) binderToDelete = null; }}>
  <Dialog.Content class="sm:max-w-[420px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-title font-semibold">Move binder to trash?</Dialog.Title>
      <Dialog.Description class="text-sm text-muted-foreground pt-1">
        <span class="font-medium text-foreground">"{binderToDelete}"</span>
        {#if binderToDelete && binderTotal(binderToDelete) > 0}
          holds {binderTotal(binderToDelete)} writing{binderTotal(binderToDelete) === 1 ? '' : 's'}.
          They go to the trash together, and one undo brings them all back.
        {:else}
          is empty. Delete it?
        {/if}
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-4 flex gap-2">
      <Button variant="ghost" onclick={() => (binderToDelete = null)} class="flex-1">Cancel</Button>
      <Button variant="destructive" onclick={confirmDeleteBinder} class="flex-1">Move to trash</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
