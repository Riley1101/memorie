<script>
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import Undo2Icon from '@lucide/svelte/icons/undo-2';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import { appState } from '$lib/runes/app.svelte.js';
  import { trashManager, RETENTION_DAYS } from '$lib/runes/trash.svelte.js';
  import { baseOf, dirOf } from '$lib/runes/fs.svelte.js';
  import { formatDate, formatTime } from '$lib/utils.js';

  /** Which row is waiting on a "delete for good" confirmation. */
  let confirmingId = $state(/** @type {string | null} */ (null));
  let isEmptying = $state(false);

  // The listing is only read while the sheet is open, so it is never stale
  // when shown and costs nothing when it isn't.
  $effect(() => {
    if (appState.ui.isTrashOpen) {
      confirmingId = null;
      trashManager.refresh();
    }
  });

  /** @param {string} path */
  function label(path) {
    return baseOf(path).replace(/\.md$/i, '');
  }

  /** @param {import('$lib/runes/trash.svelte.js').TrashEntry} entry */
  async function restore(entry) {
    await trashManager.restoreWithToast(entry);
  }

  /** @param {string} id */
  async function purge(id) {
    if (confirmingId !== id) {
      confirmingId = id;
      return;
    }
    confirmingId = null;
    await trashManager.purge(id);
  }

  async function emptyAll() {
    isEmptying = true;
    try {
      await trashManager.empty();
    } finally {
      isEmptying = false;
    }
  }
</script>

<Dialog.Root
  open={appState.ui.isTrashOpen}
  onOpenChange={(v) => appState.toggleTrash(v)}
>
  <Dialog.Content class="sm:max-w-[560px] p-0 overflow-hidden" portalProps={{}}>
    <Dialog.Header class="px-6 pt-6 pb-4 border-b border-border/20">
      <Dialog.Title class="text-xl font-normal font-writer tracking-tight">
        Recently deleted
      </Dialog.Title>
      <Dialog.Description class="text-sm text-muted-foreground/70">
        Deleted writings stay here for {RETENTION_DAYS} days, then go for good.
      </Dialog.Description>
    </Dialog.Header>

    <div class="max-h-[60vh]">
      <ScrollArea class="max-h-[60vh]">
        {#if !trashManager.hasLoaded && trashManager.isLoading}
          <p class="px-6 py-10 text-center text-sm text-muted-foreground/60">Reading the trash…</p>
        {:else if trashManager.entries.length === 0}
          <div class="flex flex-col items-center gap-3 px-6 py-14 text-center">
            <div class="size-12 rounded-full bg-muted/30 flex items-center justify-center">
              <Trash2Icon strokeWidth={1.5} class="size-6 text-muted-foreground/40" />
            </div>
            <p class="font-writer text-lg">Nothing deleted</p>
            <p class="max-w-xs text-sm text-muted-foreground/60">
              Anything you delete waits here, so a mis-click is never the end of a draft.
            </p>
          </div>
        {:else}
          <ul class="p-2">
            {#each trashManager.entries as entry (entry.id)}
              {@const where = dirOf(entry.original_path)}
              <li class="flex items-center gap-3 rounded-lg px-4 py-2.5 hover:bg-muted/30">
                {#if entry.kind === 'folder'}
                  <FolderIcon strokeWidth={1.5} class="size-4 shrink-0 text-muted-foreground/50" />
                {:else}
                  <FileIcon strokeWidth={1.5} class="size-4 shrink-0 text-muted-foreground/50" />
                {/if}
                <div class="min-w-0 flex-1">
                  <p class="truncate font-writer text-base">{label(entry.original_path)}</p>
                  <p class="flex items-center gap-2 font-mono text-[0.6875rem] text-metadata">
                    {#if where}<span class="truncate">{where}</span><span class="opacity-30">•</span>{/if}
                    {#if entry.kind === 'folder'}
                      <span>{entry.item_count} writing{entry.item_count === 1 ? '' : 's'}</span>
                      <span class="opacity-30">•</span>
                    {/if}
                    <span>{formatDate(entry.deleted_at)}, {formatTime(entry.deleted_at)}</span>
                  </p>
                </div>
                <div class="flex shrink-0 items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    class="gap-1.5"
                    onclick={() => restore(entry)}
                  >
                    <Undo2Icon strokeWidth={1.75} class="size-3.5" />
                    Put back
                  </Button>
                  <Button
                    variant={confirmingId === entry.id ? 'destructive' : 'ghost'}
                    size="sm"
                    class="gap-1.5"
                    onclick={() => purge(entry.id)}
                  >
                    <Trash2Icon strokeWidth={1.75} class="size-3.5" />
                    {confirmingId === entry.id ? 'Sure?' : 'Delete'}
                  </Button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </ScrollArea>
    </div>

    {#if trashManager.entries.length > 0}
      <div class="flex items-center justify-between gap-3 border-t border-border/20 px-6 py-3">
        <p class="text-xs text-muted-foreground/60">
          {trashManager.entries.length} item{trashManager.entries.length === 1 ? '' : 's'}
        </p>
        <Button variant="ghost" size="sm" onclick={emptyAll} disabled={isEmptying}>
          {isEmptying ? 'Emptying…' : 'Empty trash'}
        </Button>
      </div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
