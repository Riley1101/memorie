<script>
  import { onMount, tick } from 'svelte';
  import { fileManager } from '@/runes/fs.svelte';
  import { treeState } from '@/runes/tree.svelte.js';
  import { naturalCollator } from '@/utils';
  import CornerDownRightIcon from '@lucide/svelte/icons/corner-down-right';
  import { gitManager } from '@/runes/git.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import MenuIcon from '@lucide/svelte/icons/menu';
  import SearchIcon from '@lucide/svelte/icons/search';
  import SettingIcon from '@lucide/svelte/icons/settings';
  import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import ScrollFade from './scroll-fade.svelte';
  import { toast } from '$lib/toast.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { MOD_KEY } from '$lib/keyboard.svelte.js';

  let { activeBinder = $bindable(null) } = $props();

  let isCreatingBinder = $state(false);
  let newBinderName = $state('');
  let isMobileSheetOpen = $state(false);

  function selectBinder(name) {
    activeBinder = activeBinder === name ? null : name;
  }

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
    isMobileSheetOpen = false;
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

  async function handlePublish() {
    await gitManager.commitAndPush(`Update writing — ${new Date().toLocaleString()}`);
    if (gitManager.pushResult === 'error') {
      toast.error('Publish failed', gitManager.error);
    } else {
      toast.success('Published to GitHub');
    }
  }

  onMount(async () => {
    if (!configManager.config) await configManager.getConfig();
    await gitManager.checkSession();
    if (gitManager.user && configManager.config?.github_repo) {
      gitManager.refreshStatus();
    }
  });

  function openSearch(onSelect) {
    onSelect?.();
    appState.toggleCommandMenu(true);
  }

  function openSettings(onSelect) {
    onSelect?.();
    goto(resolve('/settings'));
  }
</script>

{#snippet topActions(onSelect)}
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <Button
          {...props}
          onclick={() => openSearch(onSelect)}
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-foreground rounded-full"
          aria-label="Search"
        >
          <SearchIcon strokeWidth={1.5} class="size-4" />
        </Button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="bottom" portalProps={{}}>Search · {MOD_KEY}K</Tooltip.Content>
  </Tooltip.Root>
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <Button
          {...props}
          onclick={() => openSettings(onSelect)}
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-foreground rounded-full"
          aria-label="Settings"
        >
          <SettingIcon strokeWidth={1.5} class="size-4" />
        </Button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="bottom" portalProps={{}}>Settings</Tooltip.Content>
  </Tooltip.Root>
{/snippet}

{#snippet binderList(onSelect)}
  <nav class="flex flex-col gap-0.5 px-2 pb-1.5">
    <button
      onclick={() => {
        activeBinder = null;
        onSelect?.();
      }}
      class="flex items-center gap-1.5 px-2 py-1 rounded-md text-[0.8125rem] text-left transition-colors {activeBinder ===
      null
        ? 'bg-primary/10 text-primary font-medium'
        : 'text-muted-foreground/80 hover:bg-muted/30 hover:text-foreground'}"
    >
      <FileIcon strokeWidth={1.5} class="size-4 shrink-0" />
      <span class="truncate flex-1">All Writings</span>
      <span class="text-[0.6875rem] text-muted-foreground/50 tabular-nums">{fileManager.files.length}</span>
    </button>
    <div class="my-1 border-t border-border/30"></div>
    {#each fileManager.binders as binder (binder)}
      <button
        onclick={() => {
          selectBinder(binder);
          onSelect?.();
        }}
        class="flex items-center gap-1.5 px-2 py-1 rounded-md text-[0.8125rem] text-left transition-colors {activeBinder ===
        binder
          ? 'bg-primary/10 text-primary font-medium'
          : 'text-muted-foreground/80 hover:bg-muted/30 hover:text-foreground'}"
      >
        <FolderIcon strokeWidth={1.5} class="size-4 shrink-0" />
        <span class="truncate flex-1">{binder}</span>
        <span class="text-[0.6875rem] text-muted-foreground/50 tabular-nums">{binderCount(binder)}</span>
      </button>
      {#if activeBinder === binder}
        <!-- Chapters and scenes of the open binder: click jumps the tree to that folder. -->
        {#each binderFolders as folder (folder.path)}
          <button
            onclick={() => {
              jumpToFolder(folder.path);
              onSelect?.();
            }}
            title={folder.path}
            class="flex items-center gap-1.5 py-0.5 pr-2 rounded-md text-[0.75rem] text-left text-muted-foreground/70 hover:bg-muted/30 hover:text-foreground transition-colors"
            style="padding-left: {0.5 + folder.depth * 0.75}rem"
          >
            <CornerDownRightIcon strokeWidth={1.5} class="size-3 shrink-0 text-muted-foreground/40" />
            <span class="truncate flex-1">{folder.label}</span>
          </button>
        {/each}
      {/if}
    {/each}
  </nav>
{/snippet}

{#snippet binderPanel(onSelect)}
  <div class="flex items-center justify-between px-2 pb-1.5 shrink-0">
    <button
      onclick={goToManageBinders}
      class="text-[0.6875rem] font-medium text-muted-foreground/50 hover:text-foreground transition-colors"
    >
      Binders
    </button>
    <button
      onclick={startCreateBinder}
      aria-label="New Binder"
      class="text-muted-foreground/50 hover:text-foreground transition-colors"
    >
      <PlusIcon class="size-3 shrink-0" strokeWidth={1.5} />
    </button>
  </div>
  {#if isCreatingBinder}
    <div class="px-2 pb-1.5 shrink-0">
      <div class="flex items-center gap-1.5 px-2 py-1 rounded-md text-[0.8125rem]">
        <FolderIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
        <input
          bind:value={newBinderName}
          type="text"
          placeholder="Binder name"
          autofocus
          onkeydown={(e) => {
            if (e.key === 'Enter') handleCreateBinder();
            if (e.key === 'Escape') cancelCreateBinder();
          }}
          onblur={handleCreateBlur}
          class="flex-1 min-w-0 bg-transparent text-[0.8125rem] outline-none"
 />
      </div>
    </div>
  {/if}
  <ScrollFade class="min-h-0" fadeSize="h-4">
    <ScrollArea type="scroll" class="min-h-0">
      {@render binderList(onSelect)}
    </ScrollArea>
  </ScrollFade>

  {#if gitManager.user && configManager.config?.github_repo && gitManager.status.length > 0}
    <div class="px-2 pb-2 pt-2 shrink-0 border-t border-border/30">
      <button
        onclick={handlePublish}
        disabled={gitManager.isPushing}
        class="w-full flex items-center gap-1.5 px-2 py-1 rounded-md text-[0.8125rem] text-primary hover:bg-primary/10 transition-colors disabled:opacity-50"
      >
        <UploadCloudIcon strokeWidth={1.5} class="size-4 shrink-0" />
        <span
          >{gitManager.isPushing
            ? 'Publishing…'
            : `Publish (${gitManager.status.length})`}</span
        >
      </button>
    </div>
  {/if}
{/snippet}

<!-- Wide desktop only: overlaps the page container instead of taking layout space.
     Needs the centered content's side gutter to exceed the sidebar's own footprint
     (~12rem) or it'd sit on top of text, so this only kicks in at xl (1280px+).
     Narrower windows (tablet, tiled/split desktop, phone) use the sheet trigger below. -->
<aside
  class="hidden xl:flex xl:absolute xl:top-0 xl:left-0 xl:z-30 w-44 shrink-0 h-fit max-h-[calc(100%-2rem)] mb-4 ml-4 flex-col font-writer"
>
  <div class="pt-8 flex items-center justify-end gap-0.5 -mr-1">
    {@render topActions()}
  </div>
  <div class="mb-4 md:mb-8"></div>
  {@render binderPanel()}
</aside>

<!-- Below xl: trigger opens binders as a slide-in sheet (covers phone, tablet, and
     shrunk/tiled desktop windows where there isn't room for the overlay). -->
<button
  onclick={() => (isMobileSheetOpen = true)}
  class="xl:hidden fixed top-4 left-[max(1rem,var(--titlebar-inset-left,0px))] z-30 flex items-center justify-center size-9 rounded-full bg-background/90 backdrop-blur-sm border border-border/40 shadow-sm text-muted-foreground hover:text-foreground transition-colors"
>
  <MenuIcon strokeWidth={1.5} class="size-4" />
  <span class="sr-only">Open binders</span>
</button>

<Sheet.Root bind:open={isMobileSheetOpen}>
  <Sheet.Content side="left" class="w-64 font-writer flex flex-col" portalProps={{}}>
    <Sheet.Header class="pb-0 flex-row items-center justify-between">
      <button onclick={goToManageBinders} class="text-left">
        <Sheet.Title class="text-lg font-normal hover:text-primary transition-colors"
          >Binders</Sheet.Title
        >
      </button>
      <div class="flex items-center gap-0.5 pr-8">
        {@render topActions(() => (isMobileSheetOpen = false))}
      </div>
    </Sheet.Header>
    {@render binderPanel(() => (isMobileSheetOpen = false))}
  </Sheet.Content>
</Sheet.Root>
