<script>
  import { resolve } from '$app/paths';
  import { goto } from '$app/navigation';
  import Button from './ui/button/button.svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import FileIcon from '@lucide/svelte/icons/file';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import { formatFileName, formatDate, formatTime } from '@/utils';
  import { fileManager } from '@/runes/fs.svelte';
  import BinderTreeNode from './binder-tree-node.svelte';

  let {
    node,
    binder,
    depth = 0,
    onCreateFile,
    onCreateFolder,
    onDeleteFile,
    onDeleteFolder,
    draft = null,
    onDraftInput,
    onDraftConfirm,
    onDraftCancel
  } = $props();

  let open = $state(true);
  let isRenaming = $state(false);
  let renameValue = $state('');
  let isRenamingFolder = $state(false);
  let folderRenameValue = $state('');
  let fileClickTimer = null;

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
    await fileManager.renameFile(node.file.name, renameValue.trim());
    cancelRenameFile();
  }

  function handleFileClick(e) {
    if (e.metaKey || e.ctrlKey || e.shiftKey) return;
    e.preventDefault();
    if (fileClickTimer) {
      clearTimeout(fileClickTimer);
      fileClickTimer = null;
      startRenameFile();
    } else {
      fileClickTimer = setTimeout(() => {
        fileClickTimer = null;
        goto(resolve(`/${encodeURIComponent(node.file.name)}`));
      }, 250);
    }
  }

  function startRenameFolder(e) {
    e.stopPropagation();
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
    const oldPath = [binder, ...node.path].join('/');
    const newPath = [binder, ...node.path.slice(0, -1), folderRenameValue.trim()].join('/');
    await fileManager.renameBinder(oldPath, newPath);
    cancelRenameFolder();
  }

  let isEmptyFolder = $derived(node.type === 'folder' && node.children.length === 0);

  function samePath(a, b) {
    return a.length === b.length && a.every((seg, i) => seg === b[i]);
  }

  let isDraftHere = $derived(draft !== null && samePath(draft.subPath, node.path));
</script>

{#if node.type === 'folder'}
  <div
    class="group flex items-center gap-1.5 mb-1 sticky top-0 py-1 bg-background/95 backdrop-blur-sm z-10"
    style="padding-left: {depth * 1.25}rem"
  >
    {#if isRenamingFolder}
      <FolderIcon class="size-3 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
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
        class="flex-1 min-w-0 bg-transparent text-[0.6875rem] font-medium outline-none"
      />
    {:else}
      <button
        onclick={() => (open = !open)}
        ondblclick={startRenameFolder}
        onkeydown={(e) => {
          if (e.key === 'F2') startRenameFolder(e);
        }}
        class="flex items-center gap-1 shrink-0 text-muted-foreground/60 hover:text-foreground transition-colors"
        aria-label={open ? 'Collapse folder' : 'Expand folder'}
      >
        {#if open}
          <ChevronDownIcon class="size-3" strokeWidth={1.5} />
        {:else}
          <ChevronRightIcon class="size-3" strokeWidth={1.5} />
        {/if}
      </button>
      <h3
        class="text-[0.6875rem] font-medium text-muted-foreground/60 bg-background pr-3 truncate"
      >
        {node.name}
      </h3>
    {/if}

    <div class="ml-auto flex items-center gap-0.5 opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity">
      <Button
        variant="ghost"
        size="icon"
        class="size-7 rounded-full"
        aria-label="New entry"
        onclick={() => onCreateFile(node.path)}
        disabled={false}
      >
        <PlusIcon class="size-3" strokeWidth={1.5} />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        class="size-7 rounded-full"
        aria-label="New folder"
        onclick={() => onCreateFolder(node.path)}
        disabled={false}
      >
        <FolderPlusIcon class="size-3" strokeWidth={1.5} />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        class="size-7 rounded-full"
        aria-label="Rename"
        onclick={startRenameFolder}
        disabled={false}
      >
        <PencilIcon class="size-3" strokeWidth={1.5} />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        class="size-7 rounded-full text-destructive/70 hover:text-destructive"
        aria-label={isEmptyFolder ? 'Delete' : 'Delete (not empty)'}
        onclick={() => onDeleteFolder(node.path)}
        disabled={!isEmptyFolder}
      >
        <Trash2Icon class="size-3" strokeWidth={1.5} />
      </Button>
    </div>
  </div>

  {#if open}
    {#if isDraftHere}
      <div
        class="flex items-center gap-2 py-1.5"
        style="padding-left: {(depth + 1) * 1.25}rem"
      >
        {#if draft.type === 'folder'}
          <FolderIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
        {:else}
          <FileIcon class="size-4 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
        {/if}
        <input
          value={draft.name}
          oninput={(e) => onDraftInput(e.currentTarget.value)}
          onkeydown={(e) => {
            if (e.key === 'Enter') onDraftConfirm();
            if (e.key === 'Escape') onDraftCancel();
          }}
          onblur={onDraftCancel}
          autofocus
          placeholder={draft.type === 'folder' ? 'Folder name' : 'Entry name'}
          class="flex-1 min-w-0 bg-transparent text-base outline-none"
        />
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
        {draft}
        {onDraftInput}
        {onDraftConfirm}
        {onDraftCancel}
      />
    {/each}
  {/if}
{:else}
  <div
    class="group relative flex items-center gap-2 py-2.5 rounded-md hover:bg-muted/20"
    style="padding-left: {depth * 1.25}rem"
  >
    {#if isRenaming}
      <div class="flex items-center gap-2 flex-1 min-w-0">
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
          class="flex-1 min-w-0 bg-transparent text-base outline-none"
        />
      </div>
    {:else}
      <a
        href={resolve(`/${encodeURIComponent(node.file.name)}`)}
        onclick={handleFileClick}
        onkeydown={(e) => {
          if (e.key === 'F2') {
            e.preventDefault();
            startRenameFile();
          }
        }}
        class="flex items-center gap-2 flex-1 min-w-0 text-base text-muted-foreground/90 hover:text-foreground transition-colors"
      >
        <FileIcon class="size-4 shrink-0" strokeWidth={1.5} />
        <span class="truncate">{formatFileName(node.name)}</span>
        <span class="ml-auto pl-3 shrink-0 hidden sm:flex items-center gap-1.5 text-xs text-muted-foreground/50 font-mono">
          <span>{formatDate(node.file.last_modified)}</span>
          <span class="opacity-30">•</span>
          <span>{formatTime(node.file.last_modified)}</span>
        </span>
      </a>
    {/if}

    <div class="flex items-center gap-0.5 opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity">
      <Button
        variant="ghost"
        size="icon"
        class="size-8 rounded-full"
        aria-label="Rename"
        onclick={startRenameFile}
        disabled={false}
      >
        <PencilIcon class="size-3.5" strokeWidth={1.5} />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        class="size-8 rounded-full text-destructive/70 hover:text-destructive"
        aria-label="Delete"
        onclick={() => onDeleteFile(node.file)}
        disabled={false}
      >
        <Trash2Icon class="size-3.5" strokeWidth={1.5} />
      </Button>
    </div>
  </div>
{/if}
