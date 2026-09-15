<script>
  import { fileManager } from '@/runes/fs.svelte';
  import { resolve } from '$app/paths';
  import { goto } from '$app/navigation';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import Button from './ui/button/button.svelte';
  import * as ContextMenu from './ui/context-menu/index.js';
  import { formatFileName, formatDate, formatTime } from '@/utils';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { SvelteDate } from 'svelte/reactivity';
  import ScrollFade from './scroll-fade.svelte';
  import BinderTreeNode from './binder-tree-node.svelte';
  import { buildFileTree } from '@/utils';

  let { activeBinder = null } = $props();

  let isDeleteDialogOpen = $state(false);
  let itemToDelete = $state(null);
  let renamingItem = $state(null);
  let renameValue = $state('');
  let itemClickTimer = null;

  let favourites = $derived(fileManager.files);

  let filteredFiles = $derived.by(() => {
    return favourites.filter((item) => activeBinder === null || item.folder === activeBinder);
  });

  // Binder selected: browse it as a chapters/scenes tree instead of a flat
  // date-grouped list.
  let showTree = $derived(activeBinder !== null);
  let fileTree = $derived.by(() =>
    showTree ? buildFileTree(favourites, activeBinder, fileManager.folders) : []
  );

  // Inline create draft: replaces the old "New Entry" / "New Folder" modals.
  // { subPath: string[], type: 'file' | 'folder', name: string } | null
  let draft = $state(null);

  export function handleTreeCreateFile(subPath) {
    draft = { subPath, type: 'file', name: '' };
  }

  export function handleTreeCreateFolder(subPath) {
    draft = { subPath, type: 'folder', name: '' };
  }

  function handleDraftInput(value) {
    if (draft) draft = { ...draft, name: value };
  }

  function handleDraftCancel() {
    draft = null;
  }

  async function handleDraftConfirm() {
    if (!draft || !draft.name.trim()) {
      draft = null;
      return;
    }
    if (draft.type === 'file') {
      await fileManager.createNewFile(draft.name, '', activeBinder, draft.subPath);
    } else {
      await fileManager.createFolder(activeBinder, draft.subPath, draft.name);
    }
    draft = null;
  }

  let isDeleteFolderDialogOpen = $state(false);
  let folderPathToDelete = $state([]);

  function handleTreeDeleteFolder(path) {
    folderPathToDelete = path;
    isDeleteFolderDialogOpen = true;
  }

  async function confirmDeleteFolder() {
    const relativePath = [activeBinder, ...folderPathToDelete].join('/');
    await fileManager.deleteFolder(relativePath);
    isDeleteFolderDialogOpen = false;
    folderPathToDelete = [];
  }

  let groupedFiles = $derived.by(() => {
    const files = filteredFiles;
    const groups = [
      { id: 'today', label: 'Today', files: [] },
      { id: 'yesterday', label: 'Yesterday', files: [] },
      { id: 'this-week', label: 'This Week', files: [] },
      { id: 'this-month', label: 'This Month', files: [] },
      { id: 'older', label: 'Older', files: [] }
    ];

    const now = new SvelteDate();
    const startOfToday = new SvelteDate(now.getFullYear(), now.getMonth(), now.getDate());
    const startOfYesterday = new SvelteDate(startOfToday);
    startOfYesterday.setDate(startOfYesterday.getDate() - 1);

    const startOfWeek = new SvelteDate(startOfToday);
    startOfWeek.setDate(startOfWeek.getDate() - startOfWeek.getDay());

    const startOfMonth = new SvelteDate(now.getFullYear(), now.getMonth(), 1);

    files.forEach(file => {
      const d = new SvelteDate(file.last_modified * 1000);
      if (d >= startOfToday) groups[0].files.push(file);
      else if (d >= startOfYesterday) groups[1].files.push(file);
      else if (d >= startOfWeek) groups[2].files.push(file);
      else if (d >= startOfMonth) groups[3].files.push(file);
      else groups[4].files.push(file);
    });

    return groups.filter(g => g.files.length > 0);
  });

  function startRename(item) {
    renamingItem = item;
    renameValue = formatFileName(item.name.split('/').pop());
  }

  function cancelRename() {
    renamingItem = null;
    renameValue = '';
  }

  async function confirmRename() {
    if (!renamingItem || !renameValue.trim()) {
      cancelRename();
      return;
    }
    await fileManager.renameFile(renamingItem.name, renameValue.trim());
    cancelRename();
  }

  function handleItemClick(e, item) {
    if (e.metaKey || e.ctrlKey || e.shiftKey) return;
    e.preventDefault();
    if (itemClickTimer) {
      clearTimeout(itemClickTimer);
      itemClickTimer = null;
      startRename(item);
    } else {
      itemClickTimer = setTimeout(() => {
        itemClickTimer = null;
        goto(resolve(`/${encodeURIComponent(item.name)}`));
      }, 250);
    }
  }

  function handleDeleteClick(item) {
    itemToDelete = item;
    isDeleteDialogOpen = true;
  }

  async function handleConfirmDelete() {
    if (itemToDelete) {
      await fileManager.deleteFile(itemToDelete.name);
      isDeleteDialogOpen = false;
      itemToDelete = null;
    }
  }

  function handleCancelDelete() {
    isDeleteDialogOpen = false;
    itemToDelete = null;
  }
</script>

<div class="flex flex-col w-full h-full font-writer">
  <ScrollFade class="flex-1 min-h-0">
  <ScrollArea type="scroll" class="h-full">
    {#if showTree}
      <div class="pb-20 pr-2">
        <div class="flex items-center justify-end gap-0.5 mb-6 sticky top-0 py-1 bg-background/95 backdrop-blur-sm z-10">
          <Button
            variant="ghost"
            size="icon-sm"
            class="rounded-full text-muted-foreground/50 hover:text-foreground"
            aria-label="New entry"
            onclick={() => handleTreeCreateFile([])}
            disabled={false}
          >
            <PlusIcon class="size-3.5" strokeWidth={1.5} />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            class="rounded-full text-muted-foreground/50 hover:text-foreground"
            aria-label="New folder"
            onclick={() => handleTreeCreateFolder([])}
            disabled={false}
          >
            <FolderPlusIcon class="size-3.5" strokeWidth={1.5} />
          </Button>
        </div>

        {#if draft && draft.subPath.length === 0}
          <div class="flex items-center gap-2 py-2.5">
            {#if draft.type === 'folder'}
              <FolderIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
            {:else}
              <FileIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
            {/if}
            <input
              value={draft.name}
              oninput={(e) => handleDraftInput(e.currentTarget.value)}
              onkeydown={(e) => {
                if (e.key === 'Enter') handleDraftConfirm();
                if (e.key === 'Escape') handleDraftCancel();
              }}
              onblur={handleDraftCancel}
              autofocus
              placeholder={draft.type === 'folder' ? 'Folder name' : 'Entry name'}
              class="flex-1 min-w-0 bg-muted rounded px-2 py-0.5 text-base outline-none"
 />
          </div>
        {/if}

        {#if fileTree.length === 0 && !draft}
          <div class="flex flex-col items-center justify-center py-32 text-center space-y-4 animate-in fade-in slide-in-from-bottom-4">
            <div class="size-12 rounded-full bg-muted/30 flex items-center justify-center mb-2">
              <FolderIcon strokeWidth={1.5} class="size-6 text-muted-foreground/40" />
            </div>
            <h3 class="text-xl font-normal">No writings in {activeBinder} yet</h3>
            <p class="text-muted-foreground/60 max-w-xs text-sm">
              Add an entry, or a folder to group entries under.
            </p>
          </div>
        {:else}
          {#each fileTree as node (node.type + ':' + node.name)}
            <BinderTreeNode
              {node}
              binder={activeBinder}
              onCreateFile={handleTreeCreateFile}
              onCreateFolder={handleTreeCreateFolder}
              onDeleteFile={handleDeleteClick}
              onDeleteFolder={handleTreeDeleteFolder}
              {draft}
              onDraftInput={handleDraftInput}
              onDraftConfirm={handleDraftConfirm}
              onDraftCancel={handleDraftCancel}
 />
          {/each}
        {/if}
      </div>
    {:else}
    <div class="space-y-6 pb-20 pr-2">
      <div class="flex items-center justify-end mb-4 sticky top-0 py-1 bg-background/95 backdrop-blur-sm z-10">
        <Button
          variant="ghost"
          size="icon-sm"
          class="rounded-full text-muted-foreground/50 hover:text-foreground"
          aria-label="New entry"
          onclick={() => handleTreeCreateFile([])}
          disabled={false}
        >
          <PlusIcon class="size-3.5" strokeWidth={1.5} />
        </Button>
      </div>
      {#if draft}
        <div class="flex items-center gap-2 py-2.5 mb-2">
          {#if draft.type === 'folder'}
            <FolderIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
          {:else}
            <FileIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
          {/if}
          <input
            value={draft.name}
            oninput={(e) => handleDraftInput(e.currentTarget.value)}
            onkeydown={(e) => {
              if (e.key === 'Enter') handleDraftConfirm();
              if (e.key === 'Escape') handleDraftCancel();
            }}
            onblur={handleDraftCancel}
            autofocus
            placeholder={draft.type === 'folder' ? 'Folder name' : 'Entry name'}
            class="flex-1 min-w-0 bg-muted rounded px-2 py-0.5 text-base outline-none"
 />
        </div>
      {/if}
      {#each groupedFiles as group (group.id)}
        <div class="relative">
          <div class="flex items-center mb-1 sticky top-0 py-1 bg-background/95 backdrop-blur-sm">
              <h3 class="text-[0.6875rem] font-medium text-muted-foreground/60 bg-background pr-3">
                  {group.label}
              </h3>
          </div>

          {#each group.files as item (item.path)}
            <ContextMenu.Root>
            <ContextMenu.Trigger>
            <div class="group relative flex items-center gap-2.5 py-2.5 rounded-md hover:bg-muted/20">
              {#if renamingItem === item}
                <div class="flex items-center gap-2 flex-1 min-w-0">
                  <FileIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
                  <input
                    value={renameValue}
                    oninput={(e) => (renameValue = e.currentTarget.value)}
                    onkeydown={(e) => {
                      if (e.key === 'Enter') { e.preventDefault(); confirmRename(); }
                      if (e.key === 'Escape') { e.preventDefault(); cancelRename(); }
                    }}
                    onblur={confirmRename}
                    autofocus
                    class="flex-1 min-w-0 bg-muted rounded px-2 py-0.5 text-base outline-none"
 />
                </div>
              {:else}
                <a
                  href={resolve(`/${encodeURIComponent(item.name)}`)}
                  onclick={(e) => handleItemClick(e, item)}
                  class="flex items-center gap-2 flex-1 min-w-0 text-base text-muted-foreground/90 hover:text-foreground transition-colors"
                >
                  <FileIcon class="size-4 shrink-0" strokeWidth={1.5} />
                  <span class="truncate">{formatFileName(item.name.split('/').pop())}</span>
                  <span class="ml-auto pl-3 shrink-0 hidden sm:flex items-center gap-1.5 text-xs text-muted-foreground/50 font-mono">
                    {#if item.folder && activeBinder === null}
                      <span class="flex items-center gap-1">
                        <FolderIcon strokeWidth={1.5} class="size-3" />
                        {item.folder}
                      </span>
                      <span class="opacity-30">•</span>
                    {/if}
                    <span>{formatDate(item.last_modified)}</span>
                    <span class="opacity-30">•</span>
                    <span>{formatTime(item.last_modified)}</span>
                  </span>
                </a>
              {/if}

            </div>
            </ContextMenu.Trigger>
            <ContextMenu.Content class="w-48">
              <ContextMenu.Item onclick={() => startRename(item)}>
                <PencilIcon class="size-3.5" strokeWidth={1.5} />
                Rename
              </ContextMenu.Item>
              <ContextMenu.Separator />
              <ContextMenu.Item variant="destructive" onclick={() => handleDeleteClick(item)}>
                <Trash2Icon class="size-3.5" strokeWidth={1.5} />
                Delete
              </ContextMenu.Item>
            </ContextMenu.Content>
            </ContextMenu.Root>
          {/each}
        </div>
      {/each}

      {#if filteredFiles.length === 0}
          <div class="flex flex-col items-center justify-center py-32 text-center space-y-4 animate-in fade-in slide-in-from-bottom-4">
              <div class="size-12 rounded-full bg-muted/30 flex items-center justify-center mb-2">
                  {#if activeBinder}
                      <FolderIcon strokeWidth={1.5} class="size-6 text-muted-foreground/40" />
                  {:else}
                      <FileIcon strokeWidth={1.5} class="size-6 text-muted-foreground/40" />
                  {/if}
              </div>
              <h3 class="text-xl font-normal">
                  {activeBinder ? `No writings in ${activeBinder} yet` : 'No writings yet'}
              </h3>
              <p class="text-muted-foreground/60 max-w-xs text-sm">
                  Your writing timeline will appear here. Start by creating your first writing above.
              </p>
              <Button onclick={() => handleTreeCreateFile([])} variant="outline" class="mt-4" disabled={false}>
                  Start writing
              </Button>
          </div>
      {/if}
    </div>
    {/if}
  </ScrollArea>
  </ScrollFade>
</div>

<Dialog.Root bind:open={isDeleteDialogOpen}>
  <Dialog.Content class="sm:max-w-[400px] font-writer" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal">Delete Writing</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
        Are you sure you want to delete <span class="font-bold text-foreground">"{itemToDelete?.name}"</span>?
        <br/>This action cannot be undone.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-6 flex gap-2">
      <Button variant="ghost" onclick={handleCancelDelete} class="flex-1" disabled={false}>Cancel</Button>
      <Button variant="destructive" onclick={handleConfirmDelete} class="flex-1" disabled={false}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={isDeleteFolderDialogOpen}>
  <Dialog.Content class="sm:max-w-[400px] font-writer" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal">Delete Folder</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
        Are you sure you want to delete
        <span class="font-bold text-foreground">"{folderPathToDelete[folderPathToDelete.length - 1]}"</span>?
        <br />This action cannot be undone.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-6 flex gap-2">
      <Button
        variant="ghost"
        onclick={() => (isDeleteFolderDialogOpen = false)}
        class="flex-1"
        disabled={false}>Cancel</Button
      >
      <Button variant="destructive" onclick={confirmDeleteFolder} class="flex-1" disabled={false}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<style>
    :global(.writing-surface) {
        scroll-behavior: smooth;
    }
</style>
