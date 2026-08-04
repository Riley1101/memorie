<script>
  import { onMount } from 'svelte';
  import { fileManager } from '@/runes/fs.svelte';
  import { gitManager } from '@/runes/git.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import MenuIcon from '@lucide/svelte/icons/menu';
  import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { Input } from '@/components/ui/input/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import Button from './ui/button/button.svelte';
  import ScrollFade from './scroll-fade.svelte';

  let { activeBinder = $bindable(null) } = $props();

  let isNewBinderDialogOpen = $state(false);
  let newBinderName = $state('');
  let isMobileSheetOpen = $state(false);

  function selectBinder(name) {
    activeBinder = activeBinder === name ? null : name;
  }

  function goToManageBinders() {
    isMobileSheetOpen = false;
    goto(resolve('/binders'));
  }

  async function handleCreateBinder() {
    if (!newBinderName.trim()) return;
    const name = newBinderName.trim();
    await fileManager.createBinder(name);
    activeBinder = name;
    newBinderName = '';
    isNewBinderDialogOpen = false;
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
      class="flex items-center gap-1.5 px-2 py-1 rounded-md text-[13px] text-left transition-colors {activeBinder ===
      null
        ? 'bg-primary/10 text-primary font-medium'
        : 'text-muted-foreground/80 hover:bg-muted/30 hover:text-foreground'}"
    >
      <FileIcon class="size-3 shrink-0" />
      <span class="truncate">All Writings</span>
    </button>
    {#each fileManager.binders as binder (binder)}
      <button
        onclick={() => {
          selectBinder(binder);
          onSelect?.();
        }}
        class="flex items-center gap-1.5 px-2 py-1 rounded-md text-[13px] text-left transition-colors {activeBinder ===
        binder
          ? 'bg-primary/10 text-primary font-medium'
          : 'text-muted-foreground/80 hover:bg-muted/30 hover:text-foreground'}"
      >
        <FolderIcon class="size-3 shrink-0" />
        <span class="truncate">{binder}</span>
      </button>
    {/each}
  </nav>
{/snippet}

{#snippet binderPanel(onSelect)}
  <div class="px-2 pb-1.5 shrink-0">
    <button
      onclick={goToManageBinders}
      class="text-[9px] font-bold uppercase tracking-widest text-muted-foreground/50 hover:text-foreground transition-colors"
    >
      Binders
    </button>
  </div>
  <ScrollFade class="min-h-0" fadeSize="h-4">
    <ScrollArea type="scroll" class="min-h-0">
      {@render binderList(onSelect)}
    </ScrollArea>
  </ScrollFade>
  <div class="p-2 shrink-0">
    <button
      onclick={() => (isNewBinderDialogOpen = true)}
      class="w-full flex items-center gap-1.5 px-2 py-1 rounded-md text-[13px] text-muted-foreground/60 hover:bg-muted/30 hover:text-foreground transition-colors"
    >
      <FolderPlusIcon class="size-3 shrink-0" />
      <span>New Binder</span>
    </button>
  </div>

  {#if gitManager.user && configManager.config?.github_repo}
    <div class="px-2 pb-2 pt-2 shrink-0 border-t border-border/30">
      <div class="flex items-center justify-between mb-1.5">
        <h2 class="text-[9px] font-bold uppercase tracking-widest text-muted-foreground/50">
          Changes
        </h2>
        {#if gitManager.status.length > 0}
          <span class="text-[9px] text-muted-foreground/50">{gitManager.status.length}</span>
        {/if}
      </div>

      {#if gitManager.status.length === 0}
        <p class="px-2 text-[11px] text-muted-foreground/40 italic">Up to date</p>
      {:else}
        <div class="flex flex-col gap-0.5 mb-2 max-h-24 overflow-y-auto">
          {#each gitManager.status.slice(0, 5) as file (file.path)}
            <span class="px-2 text-[11px] text-muted-foreground/70 truncate font-mono"
              >{file.path}</span
            >
          {/each}
          {#if gitManager.status.length > 5}
            <span class="px-2 text-[10px] text-muted-foreground/40"
              >+{gitManager.status.length - 5} more</span
            >
          {/if}
        </div>

        <button
          onclick={handlePublish}
          disabled={gitManager.isPushing}
          class="w-full flex items-center gap-1.5 px-2 py-1 rounded-md text-[13px] text-primary hover:bg-primary/10 transition-colors disabled:opacity-50"
        >
          <UploadCloudIcon class="size-3 shrink-0" />
          <span>{gitManager.isPushing ? 'Publishing…' : 'Publish'}</span>
        </button>
      {/if}
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

<Dialog.Root bind:open={isNewBinderDialogOpen}>
  <Dialog.Content class="sm:max-w-[400px] font-writer" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal">New Binder</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
        Binders group your writings into folders.
      </Dialog.Description>
    </Dialog.Header>
    <Input
      bind:value={newBinderName}
      type="text"
      placeholder="Binder name"
      class="mt-2"
      onkeydown={(e) => {
        if (e.key === 'Enter') handleCreateBinder();
      }}
    />
    <Dialog.Footer class="mt-6 flex gap-2">
      <Button
        variant="ghost"
        onclick={() => {
          isNewBinderDialogOpen = false;
          newBinderName = '';
        }}
        class="flex-1"
        disabled={false}>Cancel</Button
      >
      <Button variant="default" onclick={handleCreateBinder} class="flex-1" disabled={false}
        >Create</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
