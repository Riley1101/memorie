<script>
  import { fileManager } from '@/runes/fs.svelte';
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
  import ArrowDownWideNarrowIcon from '@lucide/svelte/icons/arrow-down-wide-narrow';
  import CheckIcon from '@lucide/svelte/icons/check';
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
  import { startNewWriting, writingUrl } from '$lib/new-writing.js';
  import { baseOf, dirOf } from '@/runes/fs.svelte';
  import { DRAG_MIME, readDragPayload } from '$lib/tree-dnd.js';
  import { toast } from '$lib/toast.js';
  import { offerUndo, RETENTION_DAYS } from '$lib/runes/trash.svelte.js';
  import { sceneCache } from '@/runes/scenes.svelte.js';
  import { compareByOrder, dirIsOrdered, clearOrder } from '@/runes/ordering.svelte.js';
  import { naturalCollator } from '@/utils';

  /**
   * Tailwind can't build class names at runtime, so each colour's classes are
   * written out here for the compiler to find.
   * @type {Record<string, { tile: string, dot: string }>}
   */
  const TINTS = {
    teal: { tile: 'bg-binder-teal text-white', dot: 'bg-binder-teal' },
    primary: { tile: 'bg-binder-clay text-white', dot: 'bg-binder-clay' },
    info: { tile: 'bg-binder-slate text-white', dot: 'bg-binder-slate' },
    success: { tile: 'bg-binder-sage text-white', dot: 'bg-binder-sage' },
    warning: { tile: 'bg-binder-ochre text-white', dot: 'bg-binder-ochre' },
    accent: { tile: 'bg-binder-plum text-white', dot: 'bg-binder-plum' },
    muted: { tile: 'bg-muted-foreground/60 text-white', dot: 'bg-muted-foreground/50' },
  };

  /**
   * `view` is chosen in the home toolbar: `list` is the dated list (or a
   * binder's chapter tree), `cards` the page-preview grid (or a binder's
   * corkboard).
   */
  let { activeBinder = null, view = 'list' } = $props();

  // Fills the shared scene cache so tree rows can show status and word count.
  // Unchanged writings cost nothing, so this is safe to re-run on any change.
  $effect(() => {
    const files = activeBinder ? fileManager.filesUnder(activeBinder) : fileManager.files;
    sceneCache.load(files).catch((e) => console.error('Could not read scene metadata:', e));
  });

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

  let binderView = $derived(view === 'cards' ? 'cards' : 'tree');

  /** True when the writer has dragged writings into an order of their own. */
  let isOrdered = $derived(Boolean(activeBinder) && dirIsOrdered(activeBinder ?? ''));

  /**
   * When a row was last edited: the time for today and yesterday (the group
   * heading already says which day), the date further back.
   * @param {string} groupId
   * @param {number} modified - unix seconds
   */
  function whenLabel(groupId, modified) {
    return groupId === 'today' || groupId === 'yesterday' ? formatTime(modified) : formatDate(modified);
  }

  /** "Essays · 2,031 words": where it lives and how long it is (C1). */
  function rowMeta(item) {
    const words = sceneCache.wordsOf(item.name);
    const parts = [];
    if (item.folder && activeBinder === null) parts.push(item.folder);
    if (words != null) parts.push(`${words.toLocaleString()} word${words === 1 ? '' : 's'}`);
    return parts.join(' · ');
  }
  let fileTree = $derived.by(() =>
    showTree ? buildFileTree(favourites, activeBinder, fileManager.folders, (a, b) => compareByOrder(a.name, b.name, (x, y) => naturalCollator.compare(baseOf(x), baseOf(y)))) : []
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
    try {
      const entry = await fileManager.deleteFolderRecursive(folderToDeleteFull);
      if (entry) offerUndo(entry);
    } finally {
      isDeletingFolder = false;
      isDeleteFolderDialogOpen = false;
      folderPathToDelete = [];
    }
  }

  /**
   * How the flat list is ordered. Date keeps the "Today / This Week" grouping;
   * the others are plain sorted lists, because grouping by date and sorting by
   * name at the same time reads as neither.
   * @type {{ id: 'date' | 'name' | 'words' | 'status', label: string }[]}
   */
  const SORTS = [
    { id: 'date', label: 'Last edited' },
    { id: 'name', label: 'Name' },
    { id: 'words', label: 'Word count' },
    { id: 'status', label: 'Status' },
  ];

  let sortBy = $state(/** @type {'date' | 'name' | 'words' | 'status'} */ (readSort()));

  function readSort() {
    try {
      const saved = localStorage.getItem('timelineSort');
      return SORTS.some((s) => s.id === saved) ? saved : 'date';
    } catch {
      return 'date';
    }
  }

  /** @param {'date' | 'name' | 'words' | 'status'} next */
  function setSort(next) {
    sortBy = next;
    try {
      localStorage.setItem('timelineSort', next);
    } catch {
      // Not remembered; fine.
    }
  }

  let sortLabel = $derived(SORTS.find((s) => s.id === sortBy)?.label ?? 'Last edited');

  /** The flat list under a sort other than date: one list, no date headings. */
  let sortedFiles = $derived.by(() => {
    const files = [...filteredFiles];
    if (sortBy === 'name') {
      files.sort((a, b) => naturalCollator.compare(baseOf(a.name), baseOf(b.name)));
    } else if (sortBy === 'words') {
      // Unread writings have no count yet; they sort last rather than as zero.
      files.sort((a, b) => (sceneCache.wordsOf(b.name) ?? -1) - (sceneCache.wordsOf(a.name) ?? -1));
    } else if (sortBy === 'status') {
      files.sort((a, b) => {
        const sa = sceneCache.statusOf(a.name);
        const sb = sceneCache.statusOf(b.name);
        if (sa === sb) return naturalCollator.compare(baseOf(a.name), baseOf(b.name));
        // Writings with no status go last; they are the ones not yet triaged.
        if (!sa) return 1;
        if (!sb) return -1;
        return naturalCollator.compare(sa, sb);
      });
    }
    return files;
  });

  let groupedFiles = $derived.by(() => {
    const files = filteredFiles;
    if (sortBy !== 'date') return [{ id: sortBy, label: sortLabel, files: sortedFiles }];
    const groups = [
      { id: 'today', label: 'Today', files: [] },
      { id: 'yesterday', label: 'Yesterday', files: [] },
      { id: 'this-week', label: 'This week', files: [] },
      { id: 'this-month', label: 'This month', files: [] },
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
    goto(writingUrl(item.name));
  }

  function handleDeleteClick(item) {
    itemToDelete = item;
    isDeleteDialogOpen = true;
  }

  async function handleConfirmDelete() {
    if (itemToDelete) {
      const entry = await fileManager.deleteFile(itemToDelete.name);
      if (entry) offerUndo(entry);
      isDeleteDialogOpen = false;
      itemToDelete = null;
    }
  }

  function handleCancelDelete() {
    isDeleteDialogOpen = false;
    itemToDelete = null;
  }

  /**
   * Tile tint for a binder: the colour the writer picked, or one derived from
   * the name. It used to special-case the literal binder "Work" and give every
   * other binder the same tint, which made them indistinguishable.
   * @param {string | null} folder
   */
  function folderTint(folder) {
    if (!folder) return { tile: 'bg-muted text-muted-foreground', dot: 'bg-muted-foreground/50' };
    return TINTS[appState.binderColour(folder)] ?? TINTS.primary;
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
        class="min-h-full pb-24 rounded-group transition-colors {isRootDragOver ? 'bg-selection/40 ring-1 ring-inset ring-primary/30' : ''}"
      >
        <div class="flex items-center justify-between gap-2 mt-2 mb-2 px-1 h-8">
          <span class="meta">
            {filteredFiles.length} writing{filteredFiles.length === 1 ? '' : 's'}{#if isRootDragOver}
              · drop to move to {activeBinder}{/if}
          </span>
          {#if isOrdered}
            <!-- Say the order is manual, and give one click back to by-name. -->
            <button
              type="button"
              onclick={() => clearOrder(activeBinder ?? '')}
              class="meta hover:text-foreground transition-colors"
              title="Sort by name again"
            >
              manual order · reset
            </button>
          {/if}
          <div class="flex items-center gap-1">
            {#if binderView === 'tree'}
              <button type="button" class="chip" onclick={() => handleTreeCreateFolder([])}>
                <FolderPlusIcon class="size-3.5" strokeWidth={1.6} />
                New folder
              </button>
            {/if}
          </div>
        </div>

        {#if draft && draft.subPath.length === 0}
          <div class="flex items-center gap-2 py-2.5">
            <span class="size-4 shrink-0"></span>
            <FolderIcon class="size-4 shrink-0 text-muted-foreground" strokeWidth={1.5} />
            <!-- These fields only appear because the writer asked to type in one (rename, new folder, new binder), so focus belongs here. -->
            <!-- svelte-ignore a11y_autofocus -->
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
              class="flex-1 min-w-0 bg-fill rounded-[6px] px-2 py-0.5 text-sm outline-none"
 />
          </div>
        {/if}

        {#if !fileManager.hasLoadedFiles}
          {@render loadingRows()}
        {:else if binderView === 'cards' && filteredFiles.length > 0}
          <BinderCards binder={activeBinder} />
        {:else if fileTree.length === 0 && !draft}
          <div class="flex flex-col items-center justify-center py-28 text-center gap-2">
            <FolderIcon strokeWidth={1.4} class="size-8 text-muted-foreground/40 mb-1" />
            <h3 class="text-title font-semibold">{activeBinder} is empty</h3>
            <p class="text-muted-foreground max-w-xs text-sm">
              Start writing here, or add a folder to group chapters and scenes.
            </p>
            <Button onclick={() => handleTreeCreateFile([])} variant="outline" class="mt-3 gap-1.5" disabled={false}>
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
    <div class="pb-24">
      <!-- The list was always by date; the other orders are the ones a writer
           reaches for when triaging rather than picking up where they left. -->
      <div class="flex items-center justify-end h-8 mt-2 -mb-6 relative z-10 pointer-events-none">
        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <button {...props} type="button" class="chip bg-transparent text-muted-foreground hover:text-foreground pointer-events-auto">
                <ArrowDownWideNarrowIcon class="size-3.5" strokeWidth={1.6} />
                {sortLabel}
              </button>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content class="w-44" align="end" portalProps={{}}>
            {#each SORTS as option (option.id)}
              <DropdownMenu.Item class="" inset={false} onclick={() => setSort(option.id)}>
                <span class="flex-1">{option.label}</span>
                {#if sortBy === option.id}
                  <CheckIcon class="size-3.5" strokeWidth={1.75} />
                {/if}
              </DropdownMenu.Item>
            {/each}
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>

      {#if !fileManager.hasLoadedFiles}
        {@render loadingRows()}
      {/if}
      {#each groupedFiles as group (group.id)}
        <section class="relative">
          <h3 class="group-head sticky top-0 z-[5] bg-page">{group.label}</h3>

          {#if view === 'cards'}
            <!-- Grid (C4): the shared card — a page preview with the opening
                 lines, and under it the title and the binder. -->
            <div class="wgrid">
              {#each group.files as item (item.path)}
                {@const tint = folderTint(item.folder)}
                {@const title = formatFileName(item.name.split('/').pop())}
                <ContextMenu.Root>
                  <ContextMenu.Trigger>
                    <a
                      href={writingUrl(item.name)}
                      onclick={(e) => handleItemClick(e, item)}
                      onkeydown={(e) => {
                        if (e.key === 'F2') {
                          e.preventDefault();
                          startRename(item);
                        }
                      }}
                      class="wcard"
                    >
                      <div aria-hidden="true" class="wcard-page">
                        <div class="font-serif text-xs font-medium mb-1.5 truncate">{title}</div>
                        <div class="font-serif text-xs leading-normal opacity-80 line-clamp-5">{sceneCache.get(item.name)?.excerpt ?? ''}</div>
                        <span class="wcard-fade"></span>
                      </div>
                      <div class="wcard-body">
                        {#if renamingItem === item}
                          <!-- svelte-ignore a11y_autofocus -->
                          <input
                            value={renameValue}
                            oninput={(e) => (renameValue = e.currentTarget.value)}
                            onclick={(e) => e.preventDefault()}
                            onkeydown={(e) => {
                              if (e.key === 'Enter') { e.preventDefault(); confirmRename(); }
                              if (e.key === 'Escape') { e.preventDefault(); cancelRename(); }
                            }}
                            onblur={confirmRename}
                            autofocus
                            aria-label="Rename writing"
                            class="block w-full min-w-0 bg-transparent text-[13.5px] font-medium outline-none shadow-[0_1px_0_0_var(--ring)]"
                          />
                        {:else}
                          <div class="wcard-title">{title}</div>
                        {/if}
                        <div class="meta flex items-center gap-1.5 mt-0.5 min-w-0">
                          <span class="binder-dot size-2 rounded-[2px] {tint.dot}"></span>
                          <span class="truncate">{item.folder || 'Loose writing'}</span>
                        </div>
                      </div>
                    </a>
                  </ContextMenu.Trigger>
                  <ContextMenu.Content class="w-52">
                    {@render itemMenu(item, ContextMenu)}
                  </ContextMenu.Content>
                </ContextMenu.Root>
              {/each}
            </div>
          {:else}
          <div>
          {#each group.files as item (item.path)}
            {@const tint = folderTint(item.folder)}
            {@const meta = rowMeta(item)}
            <ContextMenu.Root>
            <ContextMenu.Trigger>
            <!-- One shared row: the binder's colour, the title, and the time.
                  Binder and length are in the row's tooltip and its menu. -->
            <div class="wrow group/item {menuOpenFor === item.name ? 'is-open' : ''}">
              {#if renamingItem === item}
                <!-- Same box as the row it replaces: the title becomes the field,
                     so nothing below moves while renaming. -->
                <div class="flex items-center gap-3 flex-1 min-w-0">
                  <span class="wrow-mark {tint.dot}"></span>
                  <!-- These fields only appear because the writer asked to type in one (rename, new folder, new binder), so focus belongs here. -->
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    value={renameValue}
                    oninput={(e) => (renameValue = e.currentTarget.value)}
                    onkeydown={(e) => {
                      if (e.key === 'Enter') { e.preventDefault(); confirmRename(); }
                      if (e.key === 'Escape') { e.preventDefault(); cancelRename(); }
                    }}
                    onblur={confirmRename}
                    autofocus
                    aria-label="Rename writing"
                    class="block w-full min-w-0 p-0 m-0 border-0 bg-transparent text-[13.5px] font-medium leading-normal text-foreground outline-none rounded-none shadow-[0_1px_0_0_var(--ring)]"
                  />
                </div>
              {:else}
                <a
                  href={writingUrl(item.name)}
                  onclick={(e) => handleItemClick(e, item)}
                  onkeydown={(e) => {
                    if (e.key === 'F2') {
                      e.preventDefault();
                      startRename(item);
                    }
                  }}
                  class="flex items-center gap-3 flex-1 min-w-0"
                  title={meta || undefined}
                >
                  <span class="wrow-mark {tint.dot}"></span>
                  <span class="wrow-title">{formatFileName(item.name.split('/').pop())}</span>
                  <span class="wrow-meta group-hover/item:opacity-0 group-focus-within/item:opacity-0 {menuOpenFor === item.name ? 'opacity-0' : ''}">
                    {whenLabel(group.id, item.last_modified)}
                  </span>
                </a>

                <!-- Hover actions: same menu as the tree rows, no right-click needed. -->
                <div
                  class="absolute right-3 top-1/2 -translate-y-1/2 flex items-center opacity-0 group-hover/item:opacity-100 focus-within:opacity-100 transition-opacity {menuOpenFor === item.name ? 'opacity-100' : ''}"
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
                          class="tb-btn h-7 min-w-7 px-0 text-muted-foreground hover:text-foreground"
                        >
                          <EllipsisIcon strokeWidth={1.6} />
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
          {/if}
        </section>
      {/each}

      {#if fileManager.hasLoadedFiles && filteredFiles.length === 0}
          <div class="flex flex-col items-center justify-center py-28 text-center gap-2">
              <FileIcon strokeWidth={1.4} class="size-8 text-muted-foreground/40 mb-1" />
              <h3 class="text-title font-semibold">Nothing here yet</h3>
              <p class="text-muted-foreground max-w-xs text-sm">
                  Start writing and Memoire names the file from your first line. You can rename it any time.
              </p>
              <div class="mt-3 flex gap-2">
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
      <Dialog.Title class="text-title font-semibold">Move writing to trash?</Dialog.Title>
      <Dialog.Description class="text-sm text-muted-foreground pt-1">
        <span class="font-medium text-foreground">"{itemToDelete ? formatFileName(itemToDelete.name.split('/').pop()) : ''}"</span>{#if itemToDelete?.folder}
          from <span class="text-foreground">{itemToDelete.folder}</span>{/if}
        goes to the trash. You can put it back for {RETENTION_DAYS} days.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-4 flex gap-2">
      <Button variant="ghost" onclick={handleCancelDelete} class="flex-1" disabled={false}>Cancel</Button>
      <Button variant="destructive" onclick={handleConfirmDelete} class="flex-1" disabled={false}>Move to trash</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={isDeleteFolderDialogOpen}>
  <Dialog.Content class="sm:max-w-[440px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-title font-semibold">
        {folderToDeleteIsEmpty ? 'Delete folder' : 'Remove folder'}
      </Dialog.Title>
      <Dialog.Description class="text-sm text-muted-foreground pt-1">
        {#if folderToDeleteIsEmpty}
          <span class="font-medium text-foreground">"{folderPathToDelete[folderPathToDelete.length - 1]}"</span>
          is empty. Delete it?
        {:else}
          <span class="font-medium text-foreground">"{folderPathToDelete[folderPathToDelete.length - 1]}"</span>
          holds {folderToDeleteFiles} writing{folderToDeleteFiles === 1 ? '' : 's'}{#if folderToDeleteFolders > 0}&nbsp;and {folderToDeleteFolders} folder{folderToDeleteFolders === 1 ? '' : 's'}{/if}.
          Keep them and remove only the folder, or send the whole thing to the trash.
        {/if}
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-4 flex flex-col sm:flex-col gap-2">
      {#if folderToDeleteIsEmpty}
        <Button variant="destructive" onclick={confirmDeleteFolderEmpty} disabled={isDeletingFolder} class="w-full">
          Delete folder
        </Button>
      {:else}
        <Button onclick={confirmDeleteFolderLift} disabled={isDeletingFolder} class="w-full">
          {isDeletingFolder ? 'Removing folder…' : 'Remove folder, keep writings'}
        </Button>
        <Button variant="destructive" onclick={confirmDeleteFolderAll} disabled={isDeletingFolder} class="w-full">
          Trash folder and {folderToDeleteFiles} writing{folderToDeleteFiles === 1 ? '' : 's'}
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
