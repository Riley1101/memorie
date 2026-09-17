<script>
  import { onMount } from 'svelte';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import { Input } from '@/components/ui/input/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import HomeIcon from '@lucide/svelte/icons/home';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  let newBinderName = $state('');

  let renamingBinder = $state(null);
  let renameValue = $state('');

  let isDeleteDialogOpen = $state(false);
  let binderToDelete = $state(null);

  function binderWritingCount(name) {
    return fileManager.files.filter((f) => f.folder === name).length;
  }

  async function handleCreateBinder() {
    if (!newBinderName.trim()) return;
    await fileManager.createBinder(newBinderName.trim());
    newBinderName = '';
  }

  function startRename(binder) {
    renamingBinder = binder;
    renameValue = binder;
  }

  function cancelRename() {
    renamingBinder = null;
    renameValue = '';
  }

  async function confirmRename() {
    if (!renamingBinder) return;
    const oldName = renamingBinder;
    await fileManager.renameBinder(oldName, renameValue);
    renamingBinder = null;
    renameValue = '';
  }

  function handleDeleteClick(binder) {
    binderToDelete = binder;
    isDeleteDialogOpen = true;
  }

  function cancelDelete() {
    isDeleteDialogOpen = false;
    binderToDelete = null;
  }

  async function confirmDelete() {
    if (!binderToDelete || binderWritingCount(binderToDelete) > 0) return;
    await fileManager.deleteBinder(binderToDelete);
    isDeleteDialogOpen = false;
    binderToDelete = null;
  }

  onMount(async () => {
    await fileManager.getBinders();
    await fileManager.getRecents();
  });
</script>

<div class="page-container w-full h-full flex flex-col overflow-hidden">
  <div class="flex items-center justify-between mb-8">
    <h2 class="font-writer text-5xl font-normal text-heading-foreground">Binders</h2>
    <Button
      variant="ghost"
      size="icon"
      class="text-muted-foreground hover:text-foreground rounded-full transition-colors"
      onclick={() => goto(resolve('/'))}
      disabled={false}
    >
      <HomeIcon strokeWidth={1.5} class="size-5" />
    </Button>
  </div>

  <div class="flex-1 min-h-0 flex flex-col gap-6">
    <div class="flex items-center gap-2 shrink-0">
      <Input
        bind:value={newBinderName}
        type="text"
        placeholder="New binder name"
        class="max-w-xs"
        onkeydown={(e) => {
          if (e.key === 'Enter') handleCreateBinder();
        }}
 />
      <Button variant="default" class="gap-2" onclick={handleCreateBinder} disabled={false}>
        <FolderPlusIcon strokeWidth={1.5} class="size-3.5" />
        Create
      </Button>
    </div>

    {#if fileManager.errorMessage}
      <div class="p-3 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive shrink-0">
        {fileManager.errorMessage}
      </div>
    {/if}

    <ScrollFade class="flex-1 min-h-0">
      <ScrollArea type="scroll" class="w-full h-full">
        <div class="flex flex-col gap-2 pr-4 pb-4">
          {#each fileManager.binders as binder (binder)}
            {@const count = binderWritingCount(binder)}
            <div class="flex items-center gap-3 p-4 rounded-lg bg-muted/20 border border-border/50">
              <FolderIcon strokeWidth={1.5} class="size-4 text-muted-foreground/50 shrink-0" />

              {#if renamingBinder === binder}
                <Input
                  bind:value={renameValue}
                  type="text"
                  class="flex-1 h-9"
                  autofocus
                  onkeydown={(e) => {
                    if (e.key === 'Enter') confirmRename();
                    if (e.key === 'Escape') cancelRename();
                  }}
 />
                <Button variant="default" size="sm" class="text-xs h-9" onclick={confirmRename} disabled={false}
                  >Save</Button
                >
                <Button variant="ghost" size="sm" class="text-xs h-9" onclick={cancelRename} disabled={false}
                  >Cancel</Button
                >
              {:else}
                <div class="flex-1 min-w-0">
                  <p class="font-writer text-lg font-normal truncate">{binder}</p>
                  <p class="text-sm text-muted-foreground">
                    {count} writing{count === 1 ? '' : 's'}
                  </p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  class="size-9 rounded-full text-muted-foreground hover:text-foreground"
                  onclick={() => startRename(binder)}
                  disabled={false}
                >
                  <PencilIcon strokeWidth={1.5} class="size-3.5" />
                  <span class="sr-only">Rename {binder}</span>
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  class="size-9 rounded-full text-muted-foreground hover:text-destructive"
                  onclick={() => handleDeleteClick(binder)}
                  disabled={false}
                >
                  <Trash2Icon strokeWidth={1.5} class="size-3.5" />
                  <span class="sr-only">Delete {binder}</span>
                </Button>
              {/if}
            </div>
          {:else}
            <div class="flex flex-col items-center justify-center py-32 text-center gap-3">
              <FolderIcon strokeWidth={1.5} class="size-10 text-muted-foreground/30" />
              <p class="text-muted-foreground/60">No binders yet. Create one above.</p>
            </div>
          {/each}
        </div>
      </ScrollArea>
    </ScrollFade>
  </div>
</div>

<Dialog.Root bind:open={isDeleteDialogOpen}>
  <Dialog.Content class="sm:max-w-[400px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal font-writer">Delete Binder</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
        {#if binderToDelete && binderWritingCount(binderToDelete) > 0}
          <span class="font-bold text-foreground">"{binderToDelete}"</span> still has {binderWritingCount(binderToDelete)}
          writing{binderWritingCount(binderToDelete) === 1 ? '' : 's'} in it. Move or delete
          {binderWritingCount(binderToDelete) === 1 ? 'it' : 'them'} first before deleting this binder.
        {:else}
          Are you sure you want to delete <span class="font-bold text-foreground">"{binderToDelete}"</span>?
          <br />This action cannot be undone.
        {/if}
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-6 flex gap-2">
      <Button variant="ghost" onclick={cancelDelete} class="flex-1" disabled={false}>Cancel</Button>
      <Button
        variant="destructive"
        onclick={confirmDelete}
        class="flex-1"
        disabled={binderToDelete ? binderWritingCount(binderToDelete) > 0 : false}
        >Delete</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
