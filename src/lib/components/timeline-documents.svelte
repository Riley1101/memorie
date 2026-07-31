<script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { fileManager } from '@/runes/fs.svelte';
  import { resolve } from '$app/paths';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import Button from './ui/button/button.svelte';
  import { formatFileName } from '@/utils';
  import { Input } from '@/components/ui/input/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { SvelteDate } from 'svelte/reactivity';
  import { appState } from '$lib/runes/app.svelte.js';

  let keyword = $state('');
  let isDeleteDialogOpen = $state(false);
  let itemToDelete = $state(null);
  let activeBinder = $state(null);
  let isNewBinderDialogOpen = $state(false);
  let newBinderName = $state('');

  let favourites = $derived(fileManager.files);

  let filteredFiles = $derived(() => {
    return favourites.filter((item) =>
      item.name.toLowerCase().includes(keyword.toLowerCase()) &&
      (activeBinder === null || item.folder === activeBinder)
    );
  });

  let groupedFiles = $derived(() => {
    const files = filteredFiles();
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

  function createNewFile() {
    fileManager.createNewFile(keyword || 'Untitled', '', activeBinder).then(() => {
      keyword = '';
    });
  }

  function selectBinder(name) {
    activeBinder = activeBinder === name ? null : name;
  }

  async function handleCreateBinder() {
    if (!newBinderName.trim()) return;
    const name = newBinderName.trim();
    await fileManager.createBinder(name);
    activeBinder = name;
    newBinderName = '';
    isNewBinderDialogOpen = false;
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

  function formatTime(timestamp) {
    return new Date(timestamp * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function formatDate(timestamp) {
      return new Date(timestamp * 1000).toLocaleDateString([], { month: 'short', day: 'numeric' });
  }
</script>

<div class="flex flex-col md:flex-row h-full w-full font-writer">
  <!-- Sidebar: binders. Horizontal scroll strip on small screens, vertical list on md+ -->
  <aside class="w-full md:w-52 shrink-0 flex flex-col md:h-full border-b md:border-b-0 md:border-r border-border/30 pb-2 md:pb-0 md:pr-3 mb-3 md:mb-0">
    <h4 class="hidden md:block text-[10px] font-bold uppercase tracking-widest text-muted-foreground/50 px-2 mb-2 pt-1 shrink-0">
        Binders
    </h4>
    <ScrollArea type="scroll" orientation="both" class="w-full md:flex-1 md:min-h-0">
      <nav class="flex flex-row md:flex-col gap-1.5 md:gap-0.5 px-0.5 md:px-0 md:pr-2 pb-1 md:pb-0">
        <button
            onclick={() => activeBinder = null}
            class="shrink-0 flex items-center gap-2 px-3 py-1.5 md:px-2 rounded-full md:rounded-md text-sm text-left transition-colors {activeBinder === null ? 'bg-primary/10 text-primary font-medium' : 'text-muted-foreground/80 hover:bg-muted/30 hover:text-foreground'}"
        >
            <FileIcon class="size-3.5 shrink-0" />
            <span class="truncate max-w-32 md:max-w-none">All Writings</span>
        </button>
        {#each fileManager.binders as binder (binder)}
            <button
                onclick={() => selectBinder(binder)}
                class="shrink-0 flex items-center gap-2 px-3 py-1.5 md:px-2 rounded-full md:rounded-md text-sm text-left transition-colors {activeBinder === binder ? 'bg-primary/10 text-primary font-medium' : 'text-muted-foreground/80 hover:bg-muted/30 hover:text-foreground'}"
            >
                <FolderIcon class="size-3.5 shrink-0" />
                <span class="truncate max-w-32 md:max-w-none">{binder}</span>
            </button>
        {/each}
        <button
            onclick={() => isNewBinderDialogOpen = true}
            class="shrink-0 flex items-center gap-2 px-3 py-1.5 md:px-2 rounded-full md:rounded-md text-sm text-muted-foreground/60 hover:bg-muted/30 hover:text-foreground transition-colors border border-dashed border-border/40 md:border-none"
        >
            <FolderPlusIcon class="size-3.5 shrink-0" />
            <span>New Binder</span>
        </button>
      </nav>
    </ScrollArea>
  </aside>

  <!-- Main: files in the selected binder (or all) -->
  <div class="flex-1 min-w-0 min-h-0 h-full flex flex-col md:pl-6">
    <div class="sticky top-0 bg-background/95 backdrop-blur-sm z-20 pt-1 pb-6 shrink-0">
        <div class="relative group w-full">
            <Input
                bind:value={keyword}
                type="text"
                placeholder="Search or start something new..."
                class="pl-10 h-10 bg-muted/20 border-border/40 focus-visible:ring-1 focus-visible:ring-primary/10 transition-all text-base placeholder:text-muted-foreground/50 rounded-lg"
            />
            <FileIcon class="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/60 group-focus-within:text-primary transition-colors" />

            {#if keyword && filteredFiles().length === 0}
                <button
                    onclick={createNewFile}
                    class="absolute right-1.5 top-1/2 -translate-y-1/2 flex items-center gap-1.5 bg-primary/90 text-primary-foreground px-2.5 py-1 rounded-md text-xs font-medium hover:bg-primary transition-all animate-in fade-in scale-in-95"
                >
                    <PlusIcon class="size-3" />
                    <span>Create {keyword}.md{activeBinder ? ` in ${activeBinder}` : ''}</span>
                </button>
            {/if}
        </div>
    </div>

    <ScrollArea type="scroll" class="flex-1 min-h-0">
      <div class="space-y-12 pb-20 pr-2">
        {#each groupedFiles() as group (group.id)}
          <div class="relative">
            <div class="z-10 flex items-center gap-4 mb-6 sticky top-0 py-1 bg-background/95 backdrop-blur-sm">
                <h3 class="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 bg-background pr-3">
                    {group.label}
                </h3>
                <div class="h-px bg-border/40 flex-1"></div>
            </div>

            <div class="space-y-1">
              {#each group.files as item (item.path)}
                <div class="group relative flex gap-4 md:gap-8">
                    <!-- Timeline visual -->
                    <div class="flex flex-col items-center w-4 shrink-0">
                        <div class="size-1.5 rounded-full bg-border/60 group-hover:bg-primary/40 transition-colors mt-6"></div>
                        <div class="w-px flex-1 bg-border/20 group-last:bg-transparent"></div>
                    </div>

                    <a
                        href={resolve(`/${encodeURIComponent(item.name)}`)}
                        class="flex-1 flex items-center justify-between p-3 md:p-4 rounded-xl border border-transparent hover:border-border/40 hover:bg-muted/20 transition-all group/card overflow-hidden"
                    >
                        <div class="flex flex-col gap-0.5 truncate mr-4">
                            <span class="text-[1.1rem] md:text-xl font-normal group-hover/card:text-primary transition-colors truncate">
                                {formatFileName(item.name.split('/').pop())}
                            </span>
                            <div class="flex items-center gap-2 text-[11px] md:text-[12px] text-muted-foreground/60 font-mono">
                                {#if item.folder && activeBinder === null}
                                    <span class="flex items-center gap-1 text-muted-foreground/50">
                                        <FolderIcon class="size-2.5" />
                                        {item.folder}
                                    </span>
                                    <span class="opacity-30">•</span>
                                {/if}
                                <span>{formatDate(item.last_modified)}</span>
                                <span class="opacity-30">•</span>
                                <span>{formatTime(item.last_modified)}</span>
                            </div>
                        </div>

                        <div class="flex items-center gap-1 shrink-0 opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity">
                            <DropdownMenu.Root>
                                <DropdownMenu.Trigger>
                                    <Button variant="ghost" size="icon" class="size-7 md:size-8 rounded-full" disabled={false}>
                                        <EllipsisIcon class="size-3.5" />
                                    </Button>
                                </DropdownMenu.Trigger>
                                <DropdownMenu.Content class="{appState.ui.theme} w-40 rounded-lg" align="end" portalProps={{}}>
                                    <DropdownMenu.Item
                                        onclick={(e) => { e.preventDefault(); handleDeleteClick(item); }}
                                        variant="destructive"
                                        class="gap-2 text-[13px]"
                                        inset={false}
                                    >
                                        <Trash2Icon class="size-3.5" />
                                        <span>Delete</span>
                                    </DropdownMenu.Item>
                                </DropdownMenu.Content>
                            </DropdownMenu.Root>
                        </div>
                    </a>
                </div>
              {/each}
            </div>
          </div>
        {/each}

        {#if filteredFiles().length === 0}
            <div class="flex flex-col items-center justify-center py-32 text-center space-y-4 animate-in fade-in slide-in-from-bottom-4">
                <div class="size-12 rounded-full bg-muted/30 flex items-center justify-center mb-2">
                    {#if activeBinder}
                        <FolderIcon class="size-6 text-muted-foreground/40" />
                    {:else}
                        <FileIcon class="size-6 text-muted-foreground/40" />
                    {/if}
                </div>
                <h3 class="text-xl font-normal">
                    {activeBinder ? `No writings in ${activeBinder} yet` : 'No writings yet'}
                </h3>
                <p class="text-muted-foreground/60 max-w-xs text-sm">
                    Your writing timeline will appear here. Start by creating your first writing above.
                </p>
                <Button onclick={() => keyword = 'A new thought'} variant="outline" class="mt-4" disabled={false}>
                    Start writing
                </Button>
            </div>
        {/if}
      </div>
    </ScrollArea>
  </div>
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
        onkeydown={(e) => { if (e.key === 'Enter') handleCreateBinder(); }}
    />
    <Dialog.Footer class="mt-6 flex gap-2">
      <Button variant="ghost" onclick={() => { isNewBinderDialogOpen = false; newBinderName = ''; }} class="flex-1" disabled={false}>Cancel</Button>
      <Button variant="default" onclick={handleCreateBinder} class="flex-1" disabled={false}>Create</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<style>
    :global(.writing-surface) {
        scroll-behavior: smooth;
    }
</style>
