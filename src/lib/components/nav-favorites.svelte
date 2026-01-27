<script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import { useSidebar } from '$lib/components/ui/sidebar/index.js';
  import FileTextIcon from '@lucide/svelte/icons/file';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import { formatFileName } from '$lib/utils.js';
  import { resolve } from '$app/paths';
  import { fileManager } from '$lib/runes/fs.svelte';
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { goto } from '$app/navigation';

  /**
   * @type {  Array<{ name: string, path: string, emoji?: string }> }
   */
  let favourites = $derived(fileManager.files);

  const sidebar = useSidebar();

  let isDeleteDialogOpen = $state(false);
  let itemToDelete = $state(null);

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

  async function handleNewDocument() {
    const timestamp = new Date().getTime();
    const fileName = `untitled-${timestamp}.md`;
    await fileManager.createNewFile(fileName, "");
    goto(resolve(`/${fileName}`));
  }
</script>

<Sidebar.Group class="group-data-[collapsible=icon]:hidden">
  <Sidebar.GroupLabel class="" child={undefined}>Recents</Sidebar.GroupLabel>
  <Sidebar.GroupAction
    onclick={handleNewDocument}
    title="New Document"
    class=""
    child={undefined}
  >
    <PlusIcon />
    <span class="sr-only">New Document</span>
  </Sidebar.GroupAction>
  <Sidebar.Menu class="">
    {#each favourites as item (item.name)}
      <Sidebar.MenuItem class="">
        <Sidebar.MenuButton
          class=""
          tooltipContent={item.name}
          tooltipContentProps={{}}
          children={undefined}
        >
          {#snippet child({ props })}
            <a href={resolve(`/${item.name}`)} title={item.name} {...props}>
              <FileTextIcon class="size-4" />
              <span class="capitalize">{formatFileName(item.name)}</span>
            </a>
          {/snippet}
        </Sidebar.MenuButton>
        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <Sidebar.MenuAction showOnHover {...props}>
                <EllipsisIcon />
                <span class="sr-only">More</span>
              </Sidebar.MenuAction>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content
            class="w-56 rounded-lg"
            side={sidebar.isMobile ? 'bottom' : 'right'}
            align={sidebar.isMobile ? 'end' : 'start'}
            portalProps={{}}
          >
            <DropdownMenu.Item
              onclick={() => handleDeleteClick(item)}
              variant="destructive"
              inset={false}
              class=""
            >
              <Trash2Icon />
              <span>Delete</span>
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </Sidebar.MenuItem>
    {/each}
  </Sidebar.Menu>
</Sidebar.Group>

<Dialog.Root bind:open={isDeleteDialogOpen}>
  <Dialog.Content class="sm:max-w-[425px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="">Delete Document</Dialog.Title>
      <Dialog.Description class="text-base">
        Are you sure you want to delete <span class="font-bold text-foreground">"{itemToDelete?.name}"</span>? This action cannot be undone.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="">
      <Button variant="outline" onclick={handleCancelDelete} class="" disabled={false}>Cancel</Button>
      <Button variant="destructive" onclick={handleConfirmDelete} class="" disabled={false}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
