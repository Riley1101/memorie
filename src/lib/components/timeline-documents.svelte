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
  import * as DropdownMenu from './ui/dropdown-menu/index.js';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import FolderInputIcon from '@lucide/svelte/icons/folder-input';
  import FileInputIcon from '@lucide/svelte/icons/file-input';
  import ListTreeIcon from '@lucide/svelte/icons/list-tree';
  import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
  import BinderCards from './binder-cards.svelte';
  import { appState } from '$lib/runes/app.svelte.js';
  import { formatFileName, formatDate, formatTime } from '@/utils';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { Skeleton } from '@/components/ui/skeleton/index.js';
  import { SvelteDate } from 'svelte/reactivity';
  import { treeState } from '@/runes/tree.svelte.js';
  import ScrollFade from './scroll-fade.svelte';
  import BinderTreeNode from './binder-tree-node.svelte';
  import MovePicker from './move-picker.svelte';
  import { buildFileTree } from '@/utils';
  import { startNewWriting } from '$lib/new-writing.js';
  import { baseOf, dirOf } from '@/runes/fs.svelte';
  import { DRAG_MIME, readDragPayload } from '$lib/tree-dnd.js';
  import { toast } from '$lib/toast.js';

  let { activeBinder = null } = $props();

  // --- Move (picker + drag and drop) ---

  /** @type {{ open: boolean, kind: 'file'|'folder', subject: string, source: string, currentDir: string, excludePrefix: string|null }} */
  let move = $state({ open: false, kind: 'file', subject: '', source: '', currentDir: '', excludePrefix: null });

  function handleMoveFile(file) {
    move = {
      open: true,
      kind: 'file',
      subject: formatFileName(baseOf(file.name)),
      source: file.name,
      currentDir: dirOf(file.name),
      excludePrefix: null,
    };
  }

  function handleMoveFolder(path) {
    move = {
      open: true,
      kind: 'folder',
      subject: baseOf(path),
      source: path,
      currentDir: dirOf(path),
      excludePrefix: path,
    };
  }

  async function handleMoveChoose(dir) {
    if (move.kind === 'file') {
      const moved = await fileManager.moveFile(move.source, dir);
      if (moved) toast.success(`Moved “${move.subject}”`, dir ? `to ${dir}` : 'to top level');
    } else {
      const moved = await fileManager.moveFolder(move.source, dir);
      if (moved) {
        treeState.reveal(moved);
        toast.success(`Moved “${move.subject}”`, `to ${dir}`);
      }
    }
  }

  /** Drop target handler shared by folder rows and the binder root. */
  async function handleDrop(payload, targetDir) {
    if (payload.kind === 'file') {
      if (dirOf(payload.name) === targetDir) return;
      await fileManager.moveFile(payload.name, targetDir);
    } else {
      if (dirOf(payload.path) === targetDir) return;
      const moved = await fileManager.moveFolder(payload.path, targetDir);
      if (moved) treeState.reveal(moved);
    }
  }

  let isRootDragOver = $state(false);

  function handleRootDragOver(e) {
    if (!e.dataTransfer.types.includes(DRAG_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    isRootDragOver = true;
  }

  function handleRootDragLeave(e) {
    if (e.currentTarget.contains(e.relatedTarget)) return;
    isRootDragOver = false;
  }

  function handleRootDrop(e) {
    if (!e.dataTransfer.types.includes(DRAG_MIME)) return;
    e.preventDefault();
    isRootDragOver = false;
    const payload = readDragPayload(e);
    if (payload && activeBinder) handleDrop(payload, activeBinder);
  }

  // --- Keyboard: ↑ ↓ walk every visible row; ← → live on the folder rows. ---

  let treeContainer = $state(/** @type {HTMLElement | null} */ (null));

  function handleTreeKeydown(e) {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    if (!(e.target instanceof HTMLElement) || !e.target.hasAttribute('data-tree-row')) return;
    const rows = [...(treeContainer?.querySelectorAll('[data-tree-row]') ?? [])];
    const i = rows.indexOf(e.target);
    if (i === -1) return;
    e.preventDefault();
    const next = rows[e.key === 'ArrowDown' ? i + 1 : i - 1];
    /** @type {HTMLElement | undefined} */ (next)?.focus();
  }

  let isDeleteDialogOpen = $state(false);
  let itemToDelete = $state(null);
  let renamingItem = $state(null);
  let renameValue = $state('');
  /** Name of the writing whose hover menu is open, so its row stays highlighted. */
  let menuOpenFor = $state(/** @type {string | null} */ (null));

  let favourites = $derived(fileManager.files);

  let filteredFiles = $derived.by(() => {
    return favourites.filter((item) => activeBinder === null || item.folder === activeBinder);
  });

  // Binder selected: browse it as a chapters/scenes tree instead of a flat
  // date-grouped list.
  let showTree = $derived(activeBinder !== null);

  /** Tree or corkboard-style cards for a binder. Remembered across launches. */
  let binderView = $state(/** @type {'tree' | 'cards'} */ (readBinderView()));

  function readBinderView() {
    try {
      return localStorage.getItem('binderView') === 'cards' ? 'cards' : 'tree';
    } catch {
      return 'tree';
    }
  }

  /** @param {'tree' | 'cards'} view */
  function setBinderView(view) {
    binderView = view;
    try {
      localStorage.setItem('binderView', view);
    } catch {
      // Not remembered; fine.
    }
  }
  let fileTree = $derived.by(() =>
    showTree ? buildFileTree(favourites, activeBinder, fileManager.folders) : []
  );

  // Inline create draft, folders only. New writings open a blank editor
  // straight away and get named from what the writer types.
  // { subPath: string[], type: 'folder', name: string } | null
  let draft = $state(null);

  export function handleTreeCreateFile(subPath) {
    const dir = [activeBinder, ...subPath].filter(Boolean).join('/');
    startNewWriting(dir);
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

  let isConfirmingDraft = false;

  async function handleDraftConfirm() {
    if (isConfirmingDraft) return;
    if (!draft || !draft.name.trim()) {
      draft = null;
      return;
    }
    isConfirmingDraft = true;
    const pending = draft;
    try {
      await fileManager.createFolder(activeBinder, pending.subPath, pending.name);
      draft = null;
    } finally {
      isConfirmingDraft = false;
    }
  }

  /** Blur commits a typed name; an empty box just closes. */
  function handleDraftBlur() {
    if (draft && draft.name.trim()) handleDraftConfirm();
    else handleDraftCancel();
  }

  let isDeleteFolderDialogOpen = $state(false);
  let folderPathToDelete = $state([]);
  let isDeletingFolder = $state(false);

  let folderToDeleteFull = $derived([activeBinder, ...folderPathToDelete].join('/'));
  let folderToDeleteFiles = $derived(fileManager.filesUnder(folderToDeleteFull).length);
  let folderToDeleteFolders = $derived(fileManager.foldersUnder(folderToDeleteFull).length);
  let folderToDeleteIsEmpty = $derived(folderToDeleteFiles === 0 && folderToDeleteFolders === 0);

  function handleTreeDeleteFolder(path) {
    folderPathToDelete = path;
    isDeleteFolderDialogOpen = true;
  }

  async function confirmDeleteFolderEmpty() {
    isDeletingFolder = true;
    try {
      await fileManager.deleteFolder(folderToDeleteFull);
    } finally {
      isDeletingFolder = false;
      isDeleteFolderDialogOpen = false;
      folderPathToDelete = [];
    }
  }

  /** Default for non-empty folders: keep the writing, lose only the grouping. */
  async function confirmDeleteFolderLift() {
    isDeletingFolder = true;
    const name = baseOf(folderToDeleteFull);
    try {
      const ok = await fileManager.liftAndDeleteFolder(folderToDeleteFull);
      if (ok) toast.success(`Removed folder “${name}”`, 'Its writings moved up one level.');
    } finally {
      isDeletingFolder = false;
      isDeleteFolderDialogOpen = false;
      folderPathToDelete = [];
    }
  }

  async function confirmDeleteFolderAll() {
    isDeletingFolder = true;
    const name = baseOf(folderToDeleteFull);
    const n = folderToDeleteFiles;
    try {
      const ok = await fileManager.deleteFolderRecursive(folderToDeleteFull);
      if (ok) toast.success(`Deleted “${name}”`, `${n} writing${n === 1 ? '' : 's'} removed.`);
    } finally {
      isDeletingFolder = false;
      isDeleteFolderDialogOpen = false;
      folderPathToDelete = [];
    }
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
    const target = renamingItem;
    const value = renameValue.trim();
    cancelRename();
    await fileManager.renameFile(target.name, value);
  }

  /** Single click opens immediately; rename lives on F2 and the context menu. */
  function handleItemClick(e, item) {
    if (e.metaKey || e.ctrlKey || e.shiftKey) return;
    e.preventDefault();
    goto(resolve(`/${encodeURIComponent(item.name)}`));
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

  /** Deterministic tile tint per binder. "Work" is always teal (spec); everything
   *  else cycles through neutral/clay tints so folders stay visually distinct
   *  without every binder needing an assigned color. */
  function folderTint(folder) {
    if (!folder) return { tile: 'bg-muted text-muted-foreground', dot: 'bg-muted-foreground/50' };
    if (folder.toLowerCase() === 'work') return { tile: 'bg-teal/15 text-teal', dot: 'bg-teal' };
    return { tile: 'bg-primary/10 text-primary', dot: 'bg-primary' };
  }
</script>

{#snippet itemMenu(item, Menu)}
  <Menu.Item onclick={() => startNewWriting(dirOf(item.name))}>
    <PlusIcon class="size-3.5" strokeWidth={1.5} />
    New writing next to this
  </Menu.Item>
  <Menu.Separator />
  <Menu.Item onclick={() => startRename(item)}>
    <PencilIcon class="size-3.5" strokeWidth={1.5} />
    Rename
  </Menu.Item>
  <Menu.Item onclick={() => handleMoveFile(item)}>
    <FolderInputIcon class="size-3.5" strokeWidth={1.5} />
    Move to…
  </Menu.Item>
  <Menu.Separator />
  <Menu.Item variant="destructive" onclick={() => handleDeleteClick(item)}>
    <Trash2Icon class="size-3.5" strokeWidth={1.5} />
    Delete
  </Menu.Item>
{/snippet}

{#snippet loadingRows()}
  <div class="space-y-3 pt-2" aria-busy="true" aria-label="Loading writings">
    <Skeleton class="h-3 w-14 rounded" />
    {#each [0, 1, 2, 3] as i (i)}
      <div class="flex items-center gap-2 py-2.5">
        <Skeleton class="size-4 rounded" />
        <Skeleton class="h-4 rounded" style="width: {52 + ((i * 17) % 30)}%" />
        <Skeleton class="ml-auto h-3 w-24 rounded hidden sm:block" />
      </div>
    {/each}
  </div>
{/snippet}

<div class="flex flex-col w-full h-full">
  <ScrollFade class="flex-1 min-h-0">
  <ScrollArea type="scroll" class="h-full">
    {#if showTree}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        bind:this={treeContainer}
        onkeydown={handleTreeKeydown}
        ondragover={handleRootDragOver}
        ondragleave={handleRootDragLeave}
        ondrop={handleRootDrop}
        class="min-h-full pb-20 pr-2 rounded-lg transition-colors {isRootDragOver ? 'bg-primary/5 ring-1 ring-inset ring-primary/30' : ''}"
      >
        <div class="flex items-center justify-between gap-2 mb-4 py-1">
          <span class="text-[0.6875rem] font-mono text-metadata tabular-nums">
            {filteredFiles.length} writing{filteredFiles.length === 1 ? '' : 's'}{#if isRootDragOver}
              · drop to move to {activeBinder}{/if}
          </span>
          <div class="flex items-center gap-1">
            {#if binderView === 'tree'}
              <Button
                variant="ghost"
                size="sm"
                class="text-muted-foreground/70 hover:text-foreground gap-1.5"
                onclick={() => handleTreeCreateFolder([])}
                disabled={false}
              >
                <FolderPlusIcon class="size-3.5" strokeWidth={1.5} />
                New folder
              </Button>
            {/if}
            <div class="flex rounded-md border border-border/50 p-0.5" role="group" aria-label="View">
              <button
                type="button"
                onclick={() => setBinderView('tree')}
                aria-pressed={binderView === 'tree'}
                title="Tree"
                class="size-7 flex items-center justify-center rounded {binderView === 'tree' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}"
              >
                <ListTreeIcon class="size-3.5" strokeWidth={1.5} />
              </button>
              <button
                type="button"
                onclick={() => setBinderView('cards')}
                aria-pressed={binderView === 'cards'}
                title="Cards"
                class="size-7 flex items-center justify-center rounded {binderView === 'cards' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}"
              >
                <LayoutGridIcon class="size-3.5" strokeWidth={1.5} />
              </button>
            </div>
          </div>
        </div>

        {#if draft && draft.subPath.length === 0}
          <div class="flex items-center gap-2 py-2.5">
            <span class="size-4 shrink-0"></span>
            <FolderIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
            <input
              value={draft.name}
              oninput={(e) => handleDraftInput(e.currentTarget.value)}
              onkeydown={(e) => {
                if (e.key === 'Enter') handleDraftConfirm();
                if (e.key === 'Escape') handleDraftCancel();
              }}
              onblur={handleDraftBlur}
              autofocus
              placeholder="Folder name"
              class="flex-1 min-w-0 bg-muted rounded px-2 py-0.5 text-base outline-none"
 />
          </div>
        {/if}

        {#if !fileManager.hasLoadedFiles}
          {@render loadingRows()}
        {:else if binderView === 'cards' && filteredFiles.length > 0}
          <BinderCards binder={activeBinder} />
        {:else if fileTree.length === 0 && !draft}
          <div class="flex flex-col items-center justify-center py-32 text-center space-y-4 animate-in fade-in slide-in-from-bottom-4">
            <div class="size-12 rounded-full bg-muted/30 flex items-center justify-center mb-2">
              <FolderIcon strokeWidth={1.5} class="size-6 text-muted-foreground/40" />
            </div>
            <h3 class="font-writer text-xl font-normal">{activeBinder} is empty</h3>
            <p class="text-muted-foreground/60 max-w-xs text-sm">
              Start writing here, or add a folder to group chapters and scenes.
            </p>
            <Button onclick={() => handleTreeCreateFile([])} variant="outline" class="mt-4 gap-1.5" disabled={false}>
              <PlusIcon class="size-3.5" strokeWidth={1.5} />
              Start writing
            </Button>
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
              onMoveFile={handleMoveFile}
              onMoveFolder={handleMoveFolder}
              onDrop={handleDrop}
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
      {#if !fileManager.hasLoadedFiles}
        {@render loadingRows()}
      {/if}
      {#each groupedFiles as group (group.id)}
        <div class="relative">
          <div class="flex items-center mb-1 sticky top-0 py-1 bg-background/95 backdrop-blur-sm">
              <h3 class="text-[0.6875rem] font-medium font-mono uppercase tracking-wider text-metadata bg-background pr-3">
                  {group.label}
              </h3>
          </div>

          {#each group.files as item (item.path)}
            {@const tint = folderTint(item.folder)}
            <ContextMenu.Root>
            <ContextMenu.Trigger>
            <div
              class="group/item relative flex items-center gap-4 py-3.5 px-2 rounded-xl transition-colors hover:bg-muted/20 {menuOpenFor === item.name ? 'bg-muted/20' : ''}"
            >
              {#if renamingItem === item}
                <div class="flex items-center gap-3 flex-1 min-w-0">
                  <div class="size-10 rounded-xl {tint.tile} flex items-center justify-center shrink-0">
                    <FileIcon class="size-4" strokeWidth={1.5} />
                  </div>
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
                  onkeydown={(e) => {
                    if (e.key === 'F2') {
                      e.preventDefault();
                      startRename(item);
                    }
                  }}
                  class="flex items-center gap-4 flex-1 min-w-0"
                >
                  <div class="size-10 rounded-xl {tint.tile} flex items-center justify-center shrink-0">
                    <FileIcon class="size-4" strokeWidth={1.5} />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="truncate font-writer text-base text-foreground/90 group-hover/item:text-foreground transition-colors">
                      {formatFileName(item.name.split('/').pop())}
                    </div>
                    <div class="mt-0.5 flex items-center gap-1.5 text-[0.6875rem] font-mono text-metadata">
                      {#if item.folder && activeBinder === null}
                        <span class="flex items-center gap-1">
                          <span class="size-1.5 rounded-full {tint.dot}"></span>
                          {item.folder}
                        </span>
                        <span class="opacity-30">•</span>
                      {/if}
                      <span>{formatDate(item.last_modified)}</span>
                      <span class="opacity-30">•</span>
                      <span>{formatTime(item.last_modified)}</span>
                    </div>
                  </div>
                </a>

                <!-- Hover actions: same menu as the tree rows, no right-click needed. -->
                <div
                  class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center opacity-0 group-hover/item:opacity-100 focus-within:opacity-100 transition-opacity {menuOpenFor === item.name ? 'opacity-100' : ''}"
                >
                  <DropdownMenu.Root
                    open={menuOpenFor === item.name}
                    onOpenChange={(v) => (menuOpenFor = v ? item.name : null)}
                  >
                    <DropdownMenu.Trigger>
                      {#snippet child({ props })}
                        <button
                          {...props}
                          type="button"
                          aria-label="Actions for {formatFileName(item.name.split('/').pop())}"
                          class="flex items-center justify-center size-7 rounded-md text-muted-foreground hover:text-foreground hover:bg-foreground/5 transition-colors"
                        >
                          <EllipsisIcon class="size-3.5" strokeWidth={1.5} />
                        </button>
                      {/snippet}
                    </DropdownMenu.Trigger>
                    <DropdownMenu.Content class="w-52" align="end" portalProps={{}}>
                      {@render itemMenu(item, DropdownMenu)}
                    </DropdownMenu.Content>
                  </DropdownMenu.Root>
                </div>
              {/if}

            </div>
            </ContextMenu.Trigger>
            <ContextMenu.Content class="w-52">
              {@render itemMenu(item, ContextMenu)}
            </ContextMenu.Content>
            </ContextMenu.Root>
          {/each}
        </div>
      {/each}

      {#if fileManager.hasLoadedFiles && filteredFiles.length === 0}
          <div class="flex flex-col items-center justify-center py-32 text-center space-y-4 animate-in fade-in slide-in-from-bottom-4">
              <div class="size-12 rounded-full bg-muted/30 flex items-center justify-center mb-2">
                  <FileIcon strokeWidth={1.5} class="size-6 text-muted-foreground/40" />
              </div>
              <h3 class="font-writer text-xl font-normal">Nothing here yet</h3>
              <p class="text-muted-foreground/60 max-w-xs text-sm">
                  Start writing and Memoire names the file from your first line. You can rename it any time.
              </p>
              <div class="mt-4 flex gap-2">
                <Button onclick={() => handleTreeCreateFile([])} variant="outline" class="gap-1.5" disabled={false}>
                    <PlusIcon class="size-3.5" strokeWidth={1.5} />
                    Start writing
                </Button>
                <Button onclick={() => appState.openImport({ dir: '' })} variant="ghost" class="gap-1.5" disabled={false}>
                    <FileInputIcon class="size-3.5" strokeWidth={1.5} />
                    Import…
                </Button>
              </div>
          </div>
      {/if}
    </div>
    {/if}
  </ScrollArea>
  </ScrollFade>
</div>

<Dialog.Root bind:open={isDeleteDialogOpen}>
  <Dialog.Content class="sm:max-w-[400px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal font-writer">Delete Writing</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
        Are you sure you want to delete
        <span class="font-bold text-foreground">"{itemToDelete ? formatFileName(itemToDelete.name.split('/').pop()) : ''}"</span>{#if itemToDelete?.folder}
          from <span class="text-foreground">{itemToDelete.folder}</span>{/if}?
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
  <Dialog.Content class="sm:max-w-[440px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal font-writer">
        {folderToDeleteIsEmpty ? 'Delete folder' : 'Remove folder'}
      </Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
        {#if folderToDeleteIsEmpty}
          <span class="font-bold text-foreground">"{folderPathToDelete[folderPathToDelete.length - 1]}"</span>
          is empty. Delete it?
        {:else}
          <span class="font-bold text-foreground">"{folderPathToDelete[folderPathToDelete.length - 1]}"</span>
          holds {folderToDeleteFiles} writing{folderToDeleteFiles === 1 ? '' : 's'}{#if folderToDeleteFolders > 0}&nbsp;and {folderToDeleteFolders} folder{folderToDeleteFolders === 1 ? '' : 's'}{/if}.
          Keep them and remove only the folder, or delete everything.
        {/if}
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-6 flex flex-col sm:flex-col gap-2">
      {#if folderToDeleteIsEmpty}
        <Button variant="destructive" onclick={confirmDeleteFolderEmpty} disabled={isDeletingFolder} class="w-full">
          Delete folder
        </Button>
      {:else}
        <Button onclick={confirmDeleteFolderLift} disabled={isDeletingFolder} class="w-full">
          {isDeletingFolder ? 'Working…' : 'Remove folder, keep writings'}
        </Button>
        <Button variant="destructive" onclick={confirmDeleteFolderAll} disabled={isDeletingFolder} class="w-full">
          Delete folder and {folderToDeleteFiles} writing{folderToDeleteFiles === 1 ? '' : 's'}
        </Button>
      {/if}
      <Button
        variant="ghost"
        onclick={() => (isDeleteFolderDialogOpen = false)}
        class="w-full"
        disabled={isDeletingFolder}>Cancel</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<MovePicker
  bind:open={move.open}
  title={move.kind === 'folder' ? 'Move folder to…' : 'Move writing to…'}
  subject={move.subject}
  currentDir={move.currentDir}
  excludePrefix={move.excludePrefix}
  allowTopLevel={move.kind === 'file'}
  onChoose={handleMoveChoose}
/>

<style>
    :global(.writing-surface) {
        scroll-behavior: smooth;
    }
</style>
