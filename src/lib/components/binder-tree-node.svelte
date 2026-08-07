<script>
  import { resolve } from '$app/paths';
  import Button from './ui/button/button.svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FileIcon from '@lucide/svelte/icons/file';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import { formatFileName } from '@/utils';
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

  let isEmptyFolder = $derived(node.type === 'folder' && node.children.length === 0);

  function samePath(a, b) {
    return a.length === b.length && a.every((seg, i) => seg === b[i]);
  }

  let isDraftHere = $derived(draft !== null && samePath(draft.subPath, node.path));
</script>

{#if node.type === 'folder'}
  <div
    class="group relative flex items-center gap-2 py-1.5 rounded-md hover:bg-muted/20"
    style="padding-left: {depth * 1.25}rem"
  >
    <button
      onclick={() => (open = !open)}
      class="flex items-center gap-1.5 flex-1 min-w-0 text-left text-sm text-muted-foreground/80 hover:text-foreground transition-colors"
    >
      {#if open}
        <ChevronDownIcon class="size-3 shrink-0" strokeWidth={1.5} />
      {:else}
        <ChevronRightIcon class="size-3 shrink-0" strokeWidth={1.5} />
      {/if}
      <FolderIcon class="size-3 shrink-0" strokeWidth={1.5} />
      <span class="truncate">{node.name}</span>
    </button>

    <div class="flex items-center gap-0.5 opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity">
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
        <FolderIcon class="size-3" strokeWidth={1.5} />
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
          <FolderIcon class="size-3 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
        {:else}
          <FileIcon class="size-3 shrink-0 text-muted-foreground/60" strokeWidth={1.5} />
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
          class="flex-1 min-w-0 bg-transparent text-sm outline-none"
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
    class="group relative flex items-center gap-2 py-1.5 rounded-md hover:bg-muted/20"
    style="padding-left: {depth * 1.25}rem"
  >
    <a
      href={resolve(`/${encodeURIComponent(node.file.name)}`)}
      class="flex items-center gap-1.5 flex-1 min-w-0 text-sm text-muted-foreground/90 hover:text-foreground transition-colors"
    >
      <FileIcon class="size-3 shrink-0" strokeWidth={1.5} />
      <span class="truncate">{formatFileName(node.name)}</span>
    </a>

    <Button
      variant="ghost"
      size="icon"
      class="size-7 rounded-full text-destructive/70 hover:text-destructive opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity"
      aria-label="Delete"
      onclick={() => onDeleteFile(node.file)}
      disabled={false}
    >
      <Trash2Icon class="size-3" strokeWidth={1.5} />
    </Button>
  </div>
{/if}
