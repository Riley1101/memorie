<script>
  import { onMount } from 'svelte';
  import { fileManager } from '@/runes/fs.svelte';
  import { gitManager } from '@/runes/git.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import MenuIcon from '@lucide/svelte/icons/menu';
  import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import ScrollFade from './scroll-fade.svelte';

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

  async function handleCreateBinder() {
    if (!newBinderName.trim()) {
      cancelCreateBinder();
      return;
    }
    const name = newBinderName.trim();
    await fileManager.createBinder(name);
    activeBinder = name;
    newBinderName = '';
    isCreatingBinder = false;
  }

  async function handlePublish() {
    await gitManager.commitAndPush(`Update writing — ${new Date().toLocaleString()}`);
  }

  onMount(async () => {
    if (!configManager.config) await configManager.getConfig();
    await gitManager.checkSession();
    if (gitManager.user && configManager.config?.github_repo) {
      gitManager.refreshStatus();
    }
  });
</script>

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
      <FileIcon class="size-4 shrink-0" />
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
        <FolderIcon class="size-4 shrink-0" />
        <span class="truncate flex-1">{binder}</span>
        <span class="text-[0.6875rem] text-muted-foreground/50 tabular-nums">{binderCount(binder)}</span>
      </button>
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
          onblur={cancelCreateBinder}
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
        <UploadCloudIcon class="size-4 shrink-0" />
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
  <div class="pt-8 invisible" aria-hidden="true">
    <h2 class="text-3xl md:text-5xl font-normal">&nbsp;</h2>
  </div>
  <div class="mb-4 md:mb-8"></div>
  {@render binderPanel()}
</aside>

<!-- Below xl: trigger opens binders as a slide-in sheet (covers phone, tablet, and
     shrunk/tiled desktop windows where there isn't room for the overlay). -->
<button
  onclick={() => (isMobileSheetOpen = true)}
  class="xl:hidden fixed top-4 left-4 z-30 flex items-center justify-center size-9 rounded-full bg-background/90 backdrop-blur-sm border border-border/40 shadow-sm text-muted-foreground hover:text-foreground transition-colors"
>
  <MenuIcon class="size-4" />
  <span class="sr-only">Open binders</span>
</button>

<Sheet.Root bind:open={isMobileSheetOpen}>
  <Sheet.Content side="left" class="w-64 font-writer flex flex-col" portalProps={{}}>
    <Sheet.Header class="pb-0">
      <button onclick={goToManageBinders} class="text-left">
        <Sheet.Title class="text-lg font-normal hover:text-primary transition-colors"
          >Binders</Sheet.Title
        >
      </button>
    </Sheet.Header>
    {@render binderPanel(() => (isMobileSheetOpen = false))}
  </Sheet.Content>
</Sheet.Root>
