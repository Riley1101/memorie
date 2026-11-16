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

  /**
   * What is waiting on a "delete forever" confirmation: one entry, or the
   * whole trash. The only actions in the app that can't be undone, so both
   * ask first and say exactly what goes.
   * @type {{ kind: 'one', entry: import('$lib/runes/trash.svelte.js').TrashEntry } | { kind: 'all' } | null}
   */
  let confirming = $state(null);
  let isPurging = $state(false);
  /** @type {HTMLButtonElement | null} */
  let cancelButton = $state(null);

  $effect(() => {
    // Cancel is the safe default: Enter or a stray click should not delete.
    if (confirming) queueMicrotask(() => cancelButton?.focus());
  });

  // The listing is only read while the sheet is open, so it is never stale
  // when shown and costs nothing when it isn't.
  $effect(() => {
    if (appState.ui.isTrashOpen) {
      confirming = null;
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

  async function confirmPurge() {
    const target = confirming;
    if (!target) return;
    isPurging = true;
    try {
      if (target.kind === 'all') await trashManager.empty();
      else await trashManager.purge(target.entry.id);
    } finally {
      isPurging = false;
      confirming = null;
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
      <Dialog.Description class="text-sm text-muted-foreground">
        Deleted writings stay here for {RETENTION_DAYS} days, then go for good.
      </Dialog.Description>
    </Dialog.Header>

    <div class="max-h-[60vh]">
      <ScrollArea class="max-h-[60vh]">
        {#if !trashManager.hasLoaded && trashManager.isLoading}
          <p class="px-6 py-10 text-center text-sm text-muted-foreground">Reading the trash…</p>
        {:else if trashManager.entries.length === 0}
          <div class="flex flex-col items-center gap-3 px-6 py-14 text-center">
            <div class="size-12 rounded-full bg-muted/30 flex items-center justify-center">
              <Trash2Icon strokeWidth={1.5} class="size-6 text-muted-foreground/40" />
            </div>
            <p class="font-writer text-lg">Nothing deleted</p>
            <p class="max-w-xs text-sm text-muted-foreground">
              Anything you delete waits here, so a mis-click is never the end of a draft.
            </p>
          </div>
        {:else}
          <ul class="p-2">
            {#each trashManager.entries as entry (entry.id)}
              {@const where = dirOf(entry.original_path)}
              <li class="flex items-center gap-3 rounded-lg px-4 py-2.5 hover:bg-muted/30">
                {#if entry.kind === 'folder'}
                  <FolderIcon strokeWidth={1.5} class="size-4 shrink-0 text-muted-foreground" />
                {:else}
                  <FileIcon strokeWidth={1.5} class="size-4 shrink-0 text-muted-foreground" />
                {/if}
                <div class="min-w-0 flex-1">
                  <p class="truncate font-writer text-base">{label(entry.original_path)}</p>
                  <p class="flex items-center gap-2 tabular-nums text-meta text-metadata">
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
                    variant="ghost"
                    size="sm"
                    class="gap-1.5"
                    aria-label="Delete “{label(entry.original_path)}” forever"
                    onclick={() => (confirming = { kind: 'one', entry })}
                  >
                    <Trash2Icon strokeWidth={1.75} class="size-3.5" />
                    Delete forever
                  </Button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </ScrollArea>
    </div>

    {#if confirming}
      {@const count = trashManager.entries.length}
      <!-- Confirmation sits in the sheet's footer rather than a second modal on top of this one. -->
      <div
        class="flex flex-col gap-3 border-t border-destructive/30 bg-destructive/5 px-6 py-4 sm:flex-row sm:items-center sm:justify-between"
        role="alertdialog"
        aria-labelledby="trash-confirm-title"
      >
        <div class="min-w-0">
          <p id="trash-confirm-title" class="text-sm font-medium text-foreground">
            {#if confirming.kind === 'all'}
              Delete {count} item{count === 1 ? '' : 's'} forever?
            {:else}
              Delete “{label(confirming.entry.original_path)}” forever?
            {/if}
          </p>
          <p class="text-xs text-muted-foreground">This can’t be undone.</p>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <Button
            bind:ref={cancelButton}
            variant="ghost"
            size="sm"
            onclick={() => (confirming = null)}
            disabled={isPurging}
          >
            Cancel
          </Button>
          <Button variant="destructive" size="sm" onclick={confirmPurge} disabled={isPurging}>
            {isPurging ? 'Deleting…' : 'Delete forever'}
          </Button>
        </div>
      </div>
    {:else if trashManager.entries.length > 0}
      <div class="flex items-center justify-between gap-3 border-t border-border/20 px-6 py-3">
        <p class="text-xs text-muted-foreground">
          {trashManager.entries.length} item{trashManager.entries.length === 1 ? '' : 's'}
        </p>
        <Button variant="ghost" size="sm" onclick={() => (confirming = { kind: 'all' })}>
          Empty trash…
        </Button>
      </div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
