<script>
  import { resolve } from '$app/paths';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import Button from './ui/button/button.svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FileIcon from '@lucide/svelte/icons/file';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import { formatFileName } from '@/utils';
  import { appState } from '$lib/runes/app.svelte.js';
  import BinderTreeNode from './binder-tree-node.svelte';

  let {
    node,
    binder,
    depth = 0,
    onCreateFile,
    onCreateFolder,
    onDeleteFile,
    onDeleteFolder
  } = $props();

  let open = $state(true);

  let isEmptyFolder = $derived(node.type === 'folder' && node.children.length === 0);
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
        <ChevronDownIcon class="size-3.5 shrink-0" />
      {:else}
        <ChevronRightIcon class="size-3.5 shrink-0" />
      {/if}
      <FolderIcon class="size-3.5 shrink-0" />
      <span class="truncate">{node.name}</span>
    </button>

    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <Button
          variant="ghost"
          size="icon"
          class="size-7 rounded-full opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity"
          disabled={false}
        >
          <EllipsisIcon class="size-3.5" />
        </Button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Content class="{appState.ui.theme} w-44" align="end" portalProps={{}}>
        <DropdownMenu.Item onclick={() => onCreateFile(node.path)} class="gap-2 text-[13px]" inset={false}>
          <PlusIcon class="size-3.5" />
          <span>New entry</span>
        </DropdownMenu.Item>
        <DropdownMenu.Item onclick={() => onCreateFolder(node.path)} class="gap-2 text-[13px]" inset={false}>
          <FolderPlusIcon class="size-3.5" />
          <span>New folder</span>
        </DropdownMenu.Item>
        <DropdownMenu.Separator />
        <DropdownMenu.Item
          onclick={() => onDeleteFolder(node.path)}
          disabled={!isEmptyFolder}
          variant="destructive"
          class="gap-2 text-[13px]"
          inset={false}
        >
          <Trash2Icon class="size-3.5" />
          <span>{isEmptyFolder ? 'Delete' : 'Delete (not empty)'}</span>
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  </div>

  {#if open}
    {#each node.children as child (child.type + ':' + child.name)}
      <BinderTreeNode
        node={child}
        {binder}
        depth={depth + 1}
        {onCreateFile}
        {onCreateFolder}
        {onDeleteFile}
        {onDeleteFolder}
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
      <FileIcon class="size-3.5 shrink-0" />
      <span class="truncate">{formatFileName(node.name)}</span>
    </a>

    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <Button
          variant="ghost"
          size="icon"
          class="size-7 rounded-full opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity"
          disabled={false}
        >
          <EllipsisIcon class="size-3.5" />
        </Button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Content class="{appState.ui.theme} w-40" align="end" portalProps={{}}>
        <DropdownMenu.Item
          onclick={() => onDeleteFile(node.file)}
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
{/if}
