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
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import { codexManager } from '$lib/runes/codex.svelte.js';
  import { resolve } from '$app/paths';

  let newBinderName = $state('');

  let renamingBinder = $state(null);
  let renameValue = $state('');

  let isDeleteDialogOpen = $state(false);
  let binderToDelete = $state(null);

  function binderWritingCount(name) {
    return fileManager.files.filter((f) => f.folder === name).length;
  }

  /** Codex entries the binder itself holds (not shared ones), once loaded. */
  function binderCodexCount(name) {
    return codexManager.entities(name).filter((e) => e.scope === name).length;
  }

  /** Anything that keeps a binder from being deleted. */
  function binderBlocked(name) {
    return binderWritingCount(name) > 0 || binderCodexCount(name) > 0;
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
    codexManager.load(binder);
  }

  function cancelDelete() {
    isDeleteDialogOpen = false;
    binderToDelete = null;
  }

  async function confirmDelete() {
    if (!binderToDelete || binderBlocked(binderToDelete)) return;
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
  <div class="mb-8 shrink-0">
    <a
      href={resolve('/')}
      class="inline-flex items-center gap-1.5 -ml-1 mb-2 h-7 px-1 rounded-md text-[0.8125rem] text-muted-foreground hover:text-foreground transition-colors"
    >
      <ArrowLeftIcon strokeWidth={1.5} class="size-3.5" />
      Writings
    </a>
    <h2 class="font-writer text-5xl font-normal text-heading-foreground">Binders</h2>
  </div>

  <div class="flex-1 min-h-0 flex flex-col gap-6">
    <div class="flex flex-col gap-1.5 shrink-0">
    <label for="new-binder-name" class="text-sm text-muted-foreground">New binder</label>
    <div class="flex items-center gap-2">
      <Input
        id="new-binder-name"
        bind:value={newBinderName}
        type="text"
        placeholder="e.g. Short stories"
        class="max-w-xs"
        onkeydown={(e) => {
          if (e.key === 'Enter') handleCreateBinder();
        }}
 />
      <Button variant="default" class="gap-2" onclick={handleCreateBinder} disabled={false}>
        <FolderPlusIcon strokeWidth={1.5} class="size-3.5" />
        Create binder
      </Button>
    </div>
    </div>

    {#if fileManager.errorMessage}
      <div class="p-3 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive shrink-0">
        {fileManager.errorMessage}
        <button
          type="button"
          class="ml-2 underline underline-offset-2"
          onclick={() => Promise.all([fileManager.getRecents(), fileManager.getBinders()])}
        >
          Try again
        </button>
      </div>
    {/if}

    <ScrollFade class="flex-1 min-h-0">
      <ScrollArea type="scroll" class="w-full h-full">
        <div class="flex flex-col gap-2 pr-4 pb-4">
          {#each fileManager.binders as binder (binder)}
            {@const count = binderWritingCount(binder)}
            <div class="flex items-center gap-3 p-4 rounded-lg bg-muted/40">
              <FolderIcon strokeWidth={1.5} class="size-4 text-muted-foreground shrink-0" />

              {#if renamingBinder === binder}
                <!-- The name becomes the field in place; the count line stays, so the row keeps its height. -->
                <div class="flex-1 min-w-0">
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    bind:value={renameValue}
                    type="text"
                    aria-label="Rename binder {binder}"
                    autofocus
                    onkeydown={(e) => {
                      if (e.key === 'Enter') confirmRename();
                      if (e.key === 'Escape') cancelRename();
                    }}
                    class="block w-full min-w-0 p-0 m-0 border-0 bg-transparent font-writer text-lg font-normal text-foreground outline-none rounded-none shadow-[0_1px_0_0_var(--ring)]"
                  />
                  <p class="text-sm text-muted-foreground">
                    {count} writing{count === 1 ? '' : 's'}
                  </p>
                </div>
                <Button variant="default" size="sm" class="h-9" onclick={confirmRename} disabled={false}
                  >Save name</Button
                >
                <Button variant="ghost" size="sm" class="h-9" onclick={cancelRename} disabled={false}
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
                  class="size-9 text-muted-foreground hover:text-foreground"
                  onclick={() => startRename(binder)}
                  disabled={false}
                >
                  <PencilIcon strokeWidth={1.5} class="size-3.5" />
                  <span class="sr-only">Rename {binder}</span>
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  class="size-9 text-muted-foreground hover:text-destructive"
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
              <p class="text-muted-foreground">No binders yet. Create one above.</p>
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
      <Dialog.Title class="text-xl font-normal font-writer">Delete binder</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground pt-2">
        {#if binderToDelete && binderWritingCount(binderToDelete) > 0}
          <span class="font-medium text-foreground">"{binderToDelete}"</span> still has {binderWritingCount(binderToDelete)}
          writing{binderWritingCount(binderToDelete) === 1 ? '' : 's'} in it. Move or delete
          {binderWritingCount(binderToDelete) === 1 ? 'it' : 'them'} first before deleting this binder.
        {:else if binderToDelete && binderCodexCount(binderToDelete) > 0}
          <span class="font-medium text-foreground">"{binderToDelete}"</span> still has {binderCodexCount(binderToDelete)}
          codex entr{binderCodexCount(binderToDelete) === 1 ? 'y' : 'ies'}. Delete
          {binderCodexCount(binderToDelete) === 1 ? 'it' : 'them'} from the codex first before deleting this binder.
        {:else}
          Are you sure you want to delete <span class="font-medium text-foreground">"{binderToDelete}"</span>?
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
        disabled={binderToDelete ? binderBlocked(binderToDelete) : false}
        >Delete binder</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
