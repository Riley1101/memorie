<script>
  import { resolve } from '$app/paths';
  import { goto } from '$app/navigation';
  import * as ContextMenu from './ui/context-menu/index.js';
  import * as DropdownMenu from './ui/dropdown-menu/index.js';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import FileIcon from '@lucide/svelte/icons/file';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import FolderInputIcon from '@lucide/svelte/icons/folder-input';
  import { formatFileName, formatDate, formatTime } from '@/utils';
  import { fileManager } from '@/runes/fs.svelte';
  import { treeState } from '@/runes/tree.svelte.js';
  import { DRAG_MIME, readDragPayload } from '$lib/tree-dnd.js';
  import BinderTreeNode from './binder-tree-node.svelte';

  /**
   * Handlers all receive content-relative info. `onDrop(payload, targetDir)`
   * moves whatever was dragged into `targetDir` (a full folder path).
   */
  let {
    node,
    binder,
    depth = 0,
    onCreateFile,
    onCreateFolder,
    onDeleteFile,
    onDeleteFolder,
    onMoveFile,
    onMoveFolder,
    onDrop,
    draft = null,
    onDraftInput,
    onDraftConfirm,
    onDraftCancel
  } = $props();

  /** Full folder path for this node (folders only), e.g. "Novel/Part 1/Chapter 3". */
  let folderPath = $derived(node.type === 'folder' ? [binder, ...node.path].join('/') : '');
  let open = $derived(node.type === 'folder' ? treeState.isOpen(folderPath) : true);

  let isRenaming = $state(false);
  let renameValue = $state('');
  let isRenamingFolder = $state(false);
  let folderRenameValue = $state('');
  let isDragOver = $state(false);
  let isDragging = $state(false);
  let menuOpen = $state(false);
  let expandTimer = null;

  function startRenameFile() {
    renameValue = formatFileName(node.name);
    isRenaming = true;
  }

  function cancelRenameFile() {
    isRenaming = false;
    renameValue = '';
  }

  async function confirmRenameFile() {
    if (!renameValue.trim()) {
      cancelRenameFile();
      return;
    }
    const value = renameValue.trim();
    cancelRenameFile();
    await fileManager.renameFile(node.file.name, value);
  }

  /** Single click opens immediately; rename lives on F2 and the menus. */
  function handleFileClick(e) {
    if (e.metaKey || e.ctrlKey || e.shiftKey) return;
    e.preventDefault();
    goto(resolve(`/${encodeURIComponent(node.file.name)}`));
  }

  /** Blur commits a typed draft name; an empty box just closes. */
  function handleDraftBlur() {
    if (draft && draft.name.trim()) onDraftConfirm();
    else onDraftCancel();
  }

  function startRenameFolder(e) {
    e?.stopPropagation?.();
    folderRenameValue = node.name;
    isRenamingFolder = true;
  }

  function cancelRenameFolder() {
    isRenamingFolder = false;
    folderRenameValue = '';
  }

  async function confirmRenameFolder() {
    if (!folderRenameValue.trim() || folderRenameValue.trim() === node.name) {
      cancelRenameFolder();
      return;
    }
    const newPath = [binder, ...node.path.slice(0, -1), folderRenameValue.trim()].join('/');
    cancelRenameFolder();
    await fileManager.renameBinder(folderPath, newPath);
  }

  function samePath(a, b) {
    return a.length === b.length && a.every((seg, i) => seg === b[i]);
  }

  let isDraftHere = $derived(draft !== null && samePath(draft.subPath, node.path));

  /** ← collapses (or jumps to the parent row), → expands (or steps into the first child). */
  function handleFolderKeydown(e) {
    if (e.key === 'F2') {
      e.preventDefault();
      startRenameFolder();
    } else if (e.key === 'ArrowLeft') {
      if (open) {
        e.preventDefault();
        treeState.setOpen(folderPath, false);
      }
    } else if (e.key === 'ArrowRight') {
      if (!open) {
        e.preventDefault();
        treeState.setOpen(folderPath, true);
      }
    } else if (e.key === 'Delete' || e.key === 'Backspace') {
      if (e.metaKey || e.ctrlKey) {
        e.preventDefault();
        onDeleteFolder(node.path);
      }
    }
  }

  function handleFileKeydown(e) {
    if (e.key === 'F2') {
      e.preventDefault();
      startRenameFile();
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      onDeleteFile(node.file);
    }
  }

  // --- Drag and drop ---

  function handleDragStart(e) {
    const payload =
      node.type === 'folder'
        ? { kind: 'folder', path: folderPath, label: node.name }
        : { kind: 'file', name: node.file.name, label: formatFileName(node.name) };
    e.dataTransfer.setData(DRAG_MIME, JSON.stringify(payload));
    e.dataTransfer.effectAllowed = 'move';
    isDragging = true;
  }

  function handleDragEnd() {
    isDragging = false;
  }

  function canAccept(e) {
    return node.type === 'folder' && e.dataTransfer.types.includes(DRAG_MIME);
  }

  function handleDragOver(e) {
    if (!canAccept(e)) return;
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.dropEffect = 'move';
    if (!isDragOver) {
      isDragOver = true;
      // Hovering a closed folder for a moment opens it so you can drop deeper.
      if (!open) {
        clearTimeout(expandTimer);
        expandTimer = setTimeout(() => treeState.setOpen(folderPath, true), 600);
      }
    }
  }

  function handleDragLeave(e) {
    if (e.currentTarget.contains(e.relatedTarget)) return;
    isDragOver = false;
    clearTimeout(expandTimer);
  }

  function handleDropOnFolder(e) {
    if (!canAccept(e)) return;
    e.preventDefault();
    e.stopPropagation();
    isDragOver = false;
    clearTimeout(expandTimer);
    const payload = readDragPayload(e);
    if (payload) onDrop(payload, folderPath);
  }
</script>

{#if node.type === 'folder'}
  <ContextMenu.Root>
  <ContextMenu.Trigger>
  <div
    role="presentation"
    data-tree-folder={folderPath}
    draggable={!isRenamingFolder}
    ondragstart={handleDragStart}
    ondragend={handleDragEnd}
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDropOnFolder}
    class="group/folder relative flex items-center gap-1.5 py-2 pr-1 rounded-md transition-colors
      {isDragOver ? 'bg-primary/10 ring-1 ring-primary/40' : 'hover:bg-muted/20'}
      {isDragging ? 'opacity-40' : ''}
      {menuOpen ? 'bg-muted/20' : ''}"
    style="padding-left: {depth * 1.25}rem"
  >
    {#if isRenamingFolder}
      <span class="size-4 shrink-0"></span>
      <FolderIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
      <input
        value={folderRenameValue}
        oninput={(e) => (folderRenameValue = e.currentTarget.value)}
        onkeydown={(e) => {
          if (e.key === 'Enter') confirmRenameFolder();
          if (e.key === 'Escape') cancelRenameFolder();
        }}
        onblur={confirmRenameFolder}
        onclick={(e) => e.stopPropagation()}
        autofocus
        class="flex-1 min-w-0 bg-muted rounded px-2 py-0.5 text-base outline-none"
      />
    {:else}
      <button
        type="button"
        data-tree-row
        onclick={() => treeState.toggle(folderPath)}
        onkeydown={handleFolderKeydown}
        class="flex items-center gap-1.5 flex-1 min-w-0 text-left text-base text-foreground/90 hover:text-foreground transition-colors"
        aria-label="{open ? 'Collapse' : 'Expand'} {node.name}"
        aria-expanded={open}
      >
        <ChevronRightIcon
          class="size-4 shrink-0 text-muted-foreground/50 transition-transform duration-150 {open ? 'rotate-90' : ''}"
          strokeWidth={1.5}
        />
        {#if open}
          <FolderOpenIcon class="size-4 shrink-0 text-muted-foreground/70" strokeWidth={1.5} />
        {:else}
          <FolderIcon class="size-4 shrink-0 text-muted-foreground/70" strokeWidth={1.5} />
        {/if}
        <span class="truncate font-writer font-medium">{node.name}</span>
        <span class="ml-1 text-xs tabular-nums text-muted-foreground/50 shrink-0">
          {node.fileCount}
        </span>
        {#if node.lastModified}
          <span class="ml-auto pl-3 shrink-0 hidden sm:flex items-center gap-1.5 text-xs text-metadata font-mono group-hover/folder:opacity-0 transition-opacity">
            <span>{formatDate(node.lastModified)}</span>
            <span class="opacity-30">•</span>
            <span>{formatTime(node.lastModified)}</span>
          </span>
        {:else}
          <span class="ml-auto pl-3 shrink-0 hidden sm:inline text-xs text-muted-foreground/40 italic group-hover/folder:opacity-0 transition-opacity">
            empty
          </span>
        {/if}
      </button>

      <!-- Hover actions. Absolutely placed so the row's date can sit under them. -->
      <div
        class="absolute right-1 top-1/2 -translate-y-1/2 flex items-center gap-0.5 opacity-0 group-hover/folder:opacity-100 focus-within:opacity-100 transition-opacity
          {menuOpen ? 'opacity-100' : ''}"
      >
        <button
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            onCreateFile(node.path);
          }}
          aria-label="New writing in {node.name}"
          title="New writing here"
          class="flex items-center justify-center size-7 rounded-md text-muted-foreground hover:text-foreground hover:bg-foreground/5 transition-colors"
        >
          <PlusIcon class="size-3.5" strokeWidth={1.5} />
        </button>
        <DropdownMenu.Root bind:open={menuOpen}>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <button
                {...props}
                type="button"
                onclick={(e) => e.stopPropagation()}
                aria-label="Folder actions"
                class="flex items-center justify-center size-7 rounded-md text-muted-foreground hover:text-foreground hover:bg-foreground/5 transition-colors"
              >
                <EllipsisIcon class="size-3.5" strokeWidth={1.5} />
              </button>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content class="w-52" align="end" portalProps={{}}>
            <DropdownMenu.Item onclick={() => onCreateFile(node.path)}>
              <PlusIcon class="size-3.5" strokeWidth={1.5} />
              New writing here
            </DropdownMenu.Item>
            <DropdownMenu.Item onclick={() => onCreateFolder(node.path)}>
              <FolderPlusIcon class="size-3.5" strokeWidth={1.5} />
              New folder inside
            </DropdownMenu.Item>
            <DropdownMenu.Separator />
            <DropdownMenu.Item onclick={() => startRenameFolder()}>
              <PencilIcon class="size-3.5" strokeWidth={1.5} />
              Rename
            </DropdownMenu.Item>
            <DropdownMenu.Item onclick={() => onMoveFolder(folderPath)}>
              <FolderInputIcon class="size-3.5" strokeWidth={1.5} />
              Move to…
            </DropdownMenu.Item>
            <DropdownMenu.Separator />
            <DropdownMenu.Item variant="destructive" onclick={() => onDeleteFolder(node.path)}>
              <Trash2Icon class="size-3.5" strokeWidth={1.5} />
              Delete…
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>
    {/if}
  </div>
  </ContextMenu.Trigger>
  <ContextMenu.Content class="w-52">
    <ContextMenu.Item onclick={() => onCreateFile(node.path)}>
      <PlusIcon class="size-3.5" strokeWidth={1.5} />
      New writing here
    </ContextMenu.Item>
    <ContextMenu.Item onclick={() => onCreateFolder(node.path)}>
      <FolderPlusIcon class="size-3.5" strokeWidth={1.5} />
      New folder inside
    </ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={() => startRenameFolder()}>
      <PencilIcon class="size-3.5" strokeWidth={1.5} />
      Rename
    </ContextMenu.Item>
    <ContextMenu.Item onclick={() => onMoveFolder(folderPath)}>
      <FolderInputIcon class="size-3.5" strokeWidth={1.5} />
      Move to…
    </ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item variant="destructive" onclick={() => onDeleteFolder(node.path)}>
      <Trash2Icon class="size-3.5" strokeWidth={1.5} />
      Delete…
    </ContextMenu.Item>
  </ContextMenu.Content>
  </ContextMenu.Root>

  {#if open}
    <div class="relative">
      <!-- Guide line: makes nesting depth readable at a glance. -->
      <div
        class="absolute top-0 bottom-0 w-px bg-border/50 pointer-events-none"
        style="left: calc({depth * 1.25}rem + 0.5rem)"
      ></div>
      {#if isDraftHere}
        <div
          class="flex items-center gap-2 py-1.5"
          style="padding-left: {(depth + 1) * 1.25}rem"
        >
          <span class="size-4 shrink-0"></span>
          <FolderIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
          <input
            value={draft.name}
            oninput={(e) => onDraftInput(e.currentTarget.value)}
            onkeydown={(e) => {
              if (e.key === 'Enter') onDraftConfirm();
              if (e.key === 'Escape') onDraftCancel();
            }}
            onblur={handleDraftBlur}
            autofocus
            placeholder="Folder name"
            class="flex-1 min-w-0 bg-muted rounded px-2 py-0.5 text-base outline-none"
          />
        </div>
      {/if}
      {#if node.children.length === 0 && !isDraftHere}
        <div
          class="py-2 text-sm text-muted-foreground/40 italic select-none"
          style="padding-left: {(depth + 1) * 1.25 + 1.375}rem"
        >
          Empty. Drop writings here, or use +.
        </div>
      {/if}
      {#each node.children as child (child.type + ':' + child.name)}
        <BinderTreeNode
          node={child}
          {binder}
          depth={depth + 1}
          {onCreateFile}
          {onCreateFolder}
          {onDeleteFile}
          {onDeleteFolder}
          {onMoveFile}
          {onMoveFolder}
          {onDrop}
          {draft}
          {onDraftInput}
          {onDraftConfirm}
          {onDraftCancel}
        />
      {/each}
    </div>
  {/if}
{:else}
  <ContextMenu.Root>
  <ContextMenu.Trigger>
  <div
    role="presentation"
    draggable={!isRenaming}
    ondragstart={handleDragStart}
    ondragend={handleDragEnd}
    class="group/file relative flex items-center gap-2 py-2 pr-1 rounded-md hover:bg-muted/20 transition-colors
      {isDragging ? 'opacity-40' : ''} {menuOpen ? 'bg-muted/20' : ''}"
    style="padding-left: {depth * 1.25}rem"
  >
    {#if isRenaming}
      <div class="flex items-center gap-2 flex-1 min-w-0">
        <span class="size-4 shrink-0"></span>
        <FileIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
        <input
          value={renameValue}
          oninput={(e) => (renameValue = e.currentTarget.value)}
          onkeydown={(e) => {
            if (e.key === 'Enter') confirmRenameFile();
            if (e.key === 'Escape') cancelRenameFile();
          }}
          onblur={confirmRenameFile}
          autofocus
          class="flex-1 min-w-0 bg-muted rounded px-2 py-0.5 text-base outline-none"
        />
      </div>
    {:else}
      <a
        href={resolve(`/${encodeURIComponent(node.file.name)}`)}
        data-tree-row
        onclick={handleFileClick}
        onkeydown={handleFileKeydown}
        class="flex items-center gap-2 flex-1 min-w-0 text-base text-muted-foreground/90 hover:text-foreground transition-colors"
      >
        <span class="size-4 shrink-0"></span>
        <FileIcon class="size-4 shrink-0" strokeWidth={1.5} />
        <span class="truncate font-writer">{formatFileName(node.name)}</span>
        <span class="ml-auto pl-3 shrink-0 hidden sm:flex items-center gap-1.5 text-xs text-metadata font-mono group-hover/file:opacity-0 transition-opacity">
          <span>{formatDate(node.file.last_modified)}</span>
          <span class="opacity-30">•</span>
          <span>{formatTime(node.file.last_modified)}</span>
        </span>
      </a>

      <div
        class="absolute right-1 top-1/2 -translate-y-1/2 flex items-center opacity-0 group-hover/file:opacity-100 focus-within:opacity-100 transition-opacity
          {menuOpen ? 'opacity-100' : ''}"
      >
        <DropdownMenu.Root bind:open={menuOpen}>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <button
                {...props}
                type="button"
                aria-label="Writing actions"
                class="flex items-center justify-center size-7 rounded-md text-muted-foreground hover:text-foreground hover:bg-foreground/5 transition-colors"
              >
                <EllipsisIcon class="size-3.5" strokeWidth={1.5} />
              </button>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content class="w-52" align="end" portalProps={{}}>
            <DropdownMenu.Item onclick={() => onCreateFile(node.path ?? [])}>
              <PlusIcon class="size-3.5" strokeWidth={1.5} />
              New writing next to this
            </DropdownMenu.Item>
            <DropdownMenu.Separator />
            <DropdownMenu.Item onclick={startRenameFile}>
              <PencilIcon class="size-3.5" strokeWidth={1.5} />
              Rename
            </DropdownMenu.Item>
            <DropdownMenu.Item onclick={() => onMoveFile(node.file)}>
              <FolderInputIcon class="size-3.5" strokeWidth={1.5} />
              Move to…
            </DropdownMenu.Item>
            <DropdownMenu.Separator />
            <DropdownMenu.Item variant="destructive" onclick={() => onDeleteFile(node.file)}>
              <Trash2Icon class="size-3.5" strokeWidth={1.5} />
              Delete
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>
    {/if}
  </div>
  </ContextMenu.Trigger>
  <ContextMenu.Content class="w-52">
    <ContextMenu.Item onclick={() => onCreateFile(node.path ?? [])}>
      <PlusIcon class="size-3.5" strokeWidth={1.5} />
      New writing next to this
    </ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={startRenameFile}>
      <PencilIcon class="size-3.5" strokeWidth={1.5} />
      Rename
    </ContextMenu.Item>
    <ContextMenu.Item onclick={() => onMoveFile(node.file)}>
      <FolderInputIcon class="size-3.5" strokeWidth={1.5} />
      Move to…
    </ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item variant="destructive" onclick={() => onDeleteFile(node.file)}>
      <Trash2Icon class="size-3.5" strokeWidth={1.5} />
      Delete
    </ContextMenu.Item>
  </ContextMenu.Content>
  </ContextMenu.Root>
{/if}
