<script>
  import { onMount } from 'svelte';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import BinderSidebar from '$lib/components/binder-sidebar.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import ShareIcon from '@lucide/svelte/icons/share';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import PanelLeftIcon from '@lucide/svelte/icons/panel-left';
  import CheckIcon from '@lucide/svelte/icons/check';
  import { codexManager } from '$lib/runes/codex.svelte.js';
  import { appState, BINDER_COLOURS, BINDER_COLOUR_NAMES } from '$lib/runes/app.svelte.js';
  import { BINDER_DOT, binderDot } from '$lib/binder-tints.js';
  import { formatTimeAgo } from '$lib/utils.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  let sidebarOpen = $state(false);
  let isCreating = $state(false);

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
    const name = newBinderName.trim();
    if (!name) return;
    await fileManager.createBinder(name);
    newBinderName = '';
    isCreating = false;
  }

  function cancelCreate() {
    newBinderName = '';
    isCreating = false;
  }

  /** "12 writings · edited 2 hours ago": how much is in it and how fresh. */
  function binderMeta(name) {
    const files = fileManager.filesUnder(name);
    const n = files.length;
    const latest = files.reduce((m, f) => Math.max(m, f.last_modified), 0);
    const count = `${n} writing${n === 1 ? '' : 's'}`;
    return latest ? `${count} · edited ${formatTimeAgo(latest * 1000)}` : count;
  }

  let looseCount = $derived(fileManager.files.filter((f) => !f.folder).length);

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
    const to = renameValue.trim();
    renamingBinder = null;
    renameValue = '';
    if (!to || to === oldName) return;
    await fileManager.renameBinder(oldName, to);
    // The colour is keyed by name, so it follows the rename (as in the sidebar).
    const colour = appState.binderColours[oldName];
    if (colour) {
      appState.setBinderColour(to, colour);
      appState.setBinderColour(oldName, null);
    }
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

<div class="relative flex w-full h-full overflow-hidden gap-3 p-3">
  <BinderSidebar bind:activeBinder={appState.ui.activeBinder} bind:sheetOpen={sidebarOpen} />

  <main class="lifted flex-1 min-w-0 h-full flex flex-col rounded-group overflow-hidden">
    <header
      data-tauri-drag-region="deep"
      class="toolbar max-lg:pl-[max(20px,var(--titlebar-inset-left,0px))]"
    >
      <button
        type="button"
        class="tb-btn lg:hidden -ml-2"
        aria-label="Show library sidebar"
        onclick={() => (sidebarOpen = true)}
      >
        <PanelLeftIcon strokeWidth={1.7} />
      </button>
      <h1 class="toolbar-title">Binders</h1>
      <div class="capsule tb-primary p-0 ml-auto">
        <button type="button" class="tb-btn px-3" onclick={() => (isCreating = true)}>
          <PlusIcon strokeWidth={2} class="size-[15px]!" />
          New binder
        </button>
      </div>
    </header>

    <ScrollFade class="flex-1 min-h-0">
      <ScrollArea type="scroll" class="w-full h-full">
        <div class="mx-auto max-w-160 flex flex-col gap-8 px-8 pt-3 pb-16">
          <p class="text-sm text-muted-foreground mt-2 mx-1">
            A binder is a folder of writings: a book, a collection, a project.
          </p>

          {#if fileManager.errorMessage}
            <div class="p-3 rounded-group bg-destructive/10 text-sm text-destructive">
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

          <section>
            <h2 class="meta font-semibold mb-1.5 ml-3.5">Your binders</h2>
            <div class="inset-group">
              {#each fileManager.binders as binder (binder)}
                <div class="inset-row min-h-14">
                  <DropdownMenu.Root>
                    <DropdownMenu.Trigger>
                      {#snippet child({ props })}
                        <button
                          {...props}
                          type="button"
                          aria-label="Colour for {binder}"
                          class="size-7 shrink-0 rounded-[8px] {binderDot(
                            binder
                          )} hover:ring-2 hover:ring-hair transition-shadow"
                        ></button>
                      {/snippet}
                    </DropdownMenu.Trigger>
                    <DropdownMenu.Content class="w-40" align="start" portalProps={{}}>
                      {#each BINDER_COLOURS as colour (colour)}
                        <DropdownMenu.Item
                          class=""
                          inset={false}
                          onclick={() => appState.setBinderColour(binder, colour)}
                        >
                          <span class="binder-dot {BINDER_DOT[colour]}"></span>
                          <span class="flex-1">{BINDER_COLOUR_NAMES[colour]}</span>
                          {#if appState.binderColours[binder] === colour}<CheckIcon
                              class="size-3.5"
                            />{/if}
                        </DropdownMenu.Item>
                      {/each}
                      <DropdownMenu.Separator />
                      <DropdownMenu.Item
                        class=""
                        inset={false}
                        onclick={() => appState.setBinderColour(binder, null)}
                      >
                        From the name
                      </DropdownMenu.Item>
                    </DropdownMenu.Content>
                  </DropdownMenu.Root>

                  {#if renamingBinder === binder}
                    <!-- The name becomes the field in place; the meta line stays, so the row keeps its height. -->
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
                        class="block w-full h-6.5 px-2 -mx-2 rounded-[6px] bg-paper text-[13.5px] font-medium text-foreground outline-none ring-1 ring-primary shadow-[0_0_0_3px_color-mix(in_oklch,var(--primary)_30%,transparent)]"
                      />
                      <div class="meta mt-0.5">{binderMeta(binder)}</div>
                    </div>
                    <button type="button" class="tb-btn bg-fill" onclick={cancelRename}
                      >Cancel</button
                    >
                    <button type="button" class="tb-btn tb-primary px-3" onclick={confirmRename}
                      >Save</button
                    >
                  {:else}
                    <button
                      type="button"
                      class="flex-1 min-w-0 text-left"
                      onclick={() => {
                        appState.ui.activeBinder = binder;
                        goto(resolve('/'));
                      }}
                    >
                      <div class="text-[13.5px] font-medium truncate">{binder}</div>
                      <div class="meta truncate">{binderMeta(binder)}</div>
                    </button>
                    <button
                      type="button"
                      class="tb-btn text-muted-foreground hover:text-foreground"
                      aria-label="Export {binder}"
                      onclick={() => appState.openExport({ fileName: null, dir: binder })}
                    >
                      <ShareIcon strokeWidth={1.7} />
                    </button>
                    <DropdownMenu.Root>
                      <DropdownMenu.Trigger>
                        {#snippet child({ props })}
                          <button
                            {...props}
                            type="button"
                            class="tb-btn text-muted-foreground hover:text-foreground"
                            aria-label="More for {binder}"
                          >
                            <EllipsisIcon strokeWidth={1.7} />
                          </button>
                        {/snippet}
                      </DropdownMenu.Trigger>
                      <DropdownMenu.Content class="w-44" align="end" portalProps={{}}>
                        <DropdownMenu.Item
                          class=""
                          inset={false}
                          onclick={() => startRename(binder)}
                        >
                          <PencilIcon class="size-3.5" strokeWidth={1.5} />
                          Rename
                        </DropdownMenu.Item>
                        <DropdownMenu.Separator />
                        <DropdownMenu.Item
                          class=""
                          inset={false}
                          variant="destructive"
                          onclick={() => handleDeleteClick(binder)}
                        >
                          <Trash2Icon class="size-3.5" strokeWidth={1.5} />
                          Delete binder
                        </DropdownMenu.Item>
                      </DropdownMenu.Content>
                    </DropdownMenu.Root>
                  {/if}
                </div>
              {/each}

              {#if isCreating || fileManager.binders.length === 0}
                <!-- The new binder is typed in place, as the next row of the list. -->
                <div class="inset-row min-h-16">
                  <span
                    aria-hidden="true"
                    class="size-7 shrink-0 rounded-[8px] bg-fill flex items-center justify-center text-muted-foreground"
                  >
                    <PlusIcon class="size-3.5" strokeWidth={1.8} />
                  </span>
                  <label class="flex-1 min-w-0">
                    <span class="meta block">New binder</span>
                    <!-- svelte-ignore a11y_autofocus -->
                    <input
                      bind:value={newBinderName}
                      type="text"
                      placeholder="e.g. Poems"
                      autofocus={isCreating}
                      onkeydown={(e) => {
                        if (e.key === 'Enter') handleCreateBinder();
                        if (e.key === 'Escape') cancelCreate();
                      }}
                      class="block w-full h-6.5 mt-0.5 px-2 rounded-[6px] bg-paper text-sm text-foreground outline-none ring-1 ring-hair focus:ring-primary focus:shadow-[0_0_0_3px_color-mix(in_oklch,var(--primary)_30%,transparent)] placeholder:text-placeholder"
                    />
                  </label>
                  {#if fileManager.binders.length > 0}
                    <button type="button" class="tb-btn bg-fill" onclick={cancelCreate}
                      >Cancel</button
                    >
                  {/if}
                  <button
                    type="button"
                    class="tb-btn tb-primary px-3"
                    onclick={handleCreateBinder}
                    disabled={!newBinderName.trim()}>Create</button
                  >
                </div>
              {/if}
            </div>
          </section>

          <section>
            <h2 class="meta font-semibold mb-1.5 ml-3.5">Loose writings</h2>
            <div class="inset-group">
              <button
                type="button"
                class="inset-row min-h-14 w-full text-left hover:bg-fill transition-colors"
                onclick={() => {
                  appState.ui.activeBinder = null;
                  goto(resolve('/'));
                }}
              >
                <span aria-hidden="true" class="size-7 shrink-0 rounded-[8px] bg-fill"></span>
                <div class="flex-1 min-w-0">
                  <div class="text-[13.5px] font-medium">Not in a binder</div>
                  <div class="meta">
                    {looseCount} writing{looseCount === 1 ? '' : 's'}{looseCount
                      ? ' · move one into a binder to file it'
                      : ''}
                  </div>
                </div>
              </button>
            </div>
          </section>
        </div>
      </ScrollArea>
    </ScrollFade>
  </main>
</div>

<Dialog.Root bind:open={isDeleteDialogOpen}>
  <Dialog.Content class="sm:max-w-[400px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-title font-semibold">Delete binder</Dialog.Title>
      <Dialog.Description class="text-sm text-muted-foreground pt-1">
        {#if binderToDelete && binderWritingCount(binderToDelete) > 0}
          <span class="font-medium text-foreground">"{binderToDelete}"</span> still has {binderWritingCount(
            binderToDelete
          )}
          writing{binderWritingCount(binderToDelete) === 1 ? '' : 's'} in it. Move or delete
          {binderWritingCount(binderToDelete) === 1 ? 'it' : 'them'} first before deleting this binder.
        {:else if binderToDelete && binderCodexCount(binderToDelete) > 0}
          <span class="font-medium text-foreground">"{binderToDelete}"</span> still has {binderCodexCount(
            binderToDelete
          )}
          codex entr{binderCodexCount(binderToDelete) === 1 ? 'y' : 'ies'}. Delete
          {binderCodexCount(binderToDelete) === 1 ? 'it' : 'them'} from the codex first before deleting
          this binder.
        {:else}
          Are you sure you want to delete <span class="font-medium text-foreground"
            >"{binderToDelete}"</span
          >?
          <br />This action cannot be undone.
        {/if}
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-4 flex gap-2">
      <Button variant="ghost" onclick={cancelDelete} class="flex-1" disabled={false}>Cancel</Button>
      <Button
        variant="destructive"
        onclick={confirmDelete}
        class="flex-1"
        disabled={binderToDelete ? binderBlocked(binderToDelete) : false}>Delete binder</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
