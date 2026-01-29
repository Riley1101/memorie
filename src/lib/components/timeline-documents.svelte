<script>
  import * as Item from '$lib/components/ui/item/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { fileManager } from '@/runes/fs.svelte';
  import { resolve } from '$app/paths';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import FileIcon from '@lucide/svelte/icons/file';
  import Button from './ui/button/button.svelte';
  import { formatFileName } from '@/utils';
  import { Input } from '@/components/ui/input/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { cn } from '$lib/utils';

  let keyword = $state('');
  let isDeleteDialogOpen = $state(false);
  let itemToDelete = $state(null);

  let favourites = $derived(fileManager.files);

  let filteredFiles = $derived(() => {
    return favourites.filter((item) => 
      item.name.toLowerCase().includes(keyword.toLowerCase())
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

    const now = new Date();
    const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const startOfYesterday = new Date(startOfToday);
    startOfYesterday.setDate(startOfYesterday.getDate() - 1);
    
    const startOfWeek = new Date(startOfToday);
    startOfWeek.setDate(startOfWeek.getDate() - startOfWeek.getDay());
    
    const startOfMonth = new Date(now.getFullYear(), now.getMonth(), 1);

    files.forEach(file => {
      const d = new Date(file.last_modified * 1000);
      if (d >= startOfToday) groups[0].files.push(file);
      else if (d >= startOfYesterday) groups[1].files.push(file);
      else if (d >= startOfWeek) groups[2].files.push(file);
      else if (d >= startOfMonth) groups[3].files.push(file);
      else groups[4].files.push(file);
    });

    return groups.filter(g => g.files.length > 0);
  });

  function createNewFile() {
    fileManager.createNewFile(keyword || 'Untitled').then(() => {
      keyword = '';
    });
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

<div class="flex flex-col w-full pb-20 font-writer">
  <div class="sticky top-0 bg-background/95 backdrop-blur-sm z-20 pt-4 pb-6">
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
                <span>Create {keyword}.md</span>
            </button>
        {/if}
    </div>
  </div>

  <div class="space-y-12 mt-4">
    {#each groupedFiles() as group (group.id)}
      <div class="relative">
        <div class="z-10 flex items-center gap-4 mb-6 sticky top-18 py-1 bg-background/95 backdrop-blur-sm">
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
                    href={resolve(`/${item.name}`)} 
                    class="flex-1 flex items-center justify-between p-3 md:p-4 rounded-xl border border-transparent hover:border-border/40 hover:bg-muted/20 transition-all group/card overflow-hidden"
                >
                    <div class="flex flex-col gap-0.5 truncate mr-4">
                        <span class="text-[1.1rem] md:text-xl font-normal group-hover/card:text-primary transition-colors truncate">
                            {formatFileName(item.name)}
                        </span>
                        <div class="flex items-center gap-2 text-[11px] md:text-[12px] text-muted-foreground/60 font-mono">
                            <span>{formatDate(item.last_modified)}</span>
                            <span class="opacity-30">•</span>
                            <span>{formatTime(item.last_modified)}</span>
                        </div>
                    </div>

                    <div class="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                        <DropdownMenu.Root>
                            <DropdownMenu.Trigger>
                                <Button variant="ghost" size="icon" class="size-7 md:size-8 rounded-full" disabled={false}>
                                    <EllipsisIcon class="size-3.5" />
                                </Button>
                            </DropdownMenu.Trigger>
                            <DropdownMenu.Content class="dark w-40 rounded-lg" align="end" portalProps={{}}>
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

    {#if favourites.length === 0}
        <div class="flex flex-col items-center justify-center py-32 text-center space-y-4 animate-in fade-in slide-in-from-bottom-4">
            <div class="size-12 rounded-full bg-muted/30 flex items-center justify-center mb-2">
                <FileIcon class="size-6 text-muted-foreground/40" />
            </div>
            <h3 class="text-xl font-normal">No documents yet</h3>
            <p class="text-muted-foreground/60 max-w-xs text-sm">
                Your writing timeline will appear here. Start by creating your first document above.
            </p>
            <Button onclick={() => keyword = 'A new thought'} variant="outline" class="mt-4" disabled={false}>
                Start writing
            </Button>
        </div>
    {/if}
  </div>
</div>

<Dialog.Root bind:open={isDeleteDialogOpen}>
  <Dialog.Content class="sm:max-w-[400px] font-writer" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal">Delete Document</Dialog.Title>
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

<style>
    :global(.writing-surface) {
        scroll-behavior: smooth;
    }
</style>
