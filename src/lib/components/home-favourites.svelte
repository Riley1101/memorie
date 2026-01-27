<script>
  import * as Item from '$lib/components/ui/item/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { fileManager } from '@/runes/fs.svelte';
  import { resolve } from '$app/paths';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import FileIcon from '@lucide/svelte/icons/file';
  import Button from './ui/button/button.svelte';
  import { formatFileName } from '@/utils';
  import { Input } from '@/components/ui/input/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';

  /**
   * @type {{
   *   limit?:number
   * }}
   */
  let { limit: propsLimit = 5 } = $props();

  let favourites = $derived(fileManager.files);

  let keyword = $state('');
  let limit = $derived(propsLimit);
  let isDeleteDialogOpen = $state(false);
  let itemToDelete = $state(null);

  let filteredFavourites = $derived(() => {
    return favourites
      .filter((item) => item.name.toLowerCase().includes(keyword.toLowerCase()))
      .slice(0, limit);
  });

  function loadMore() {
    limit += 4;
  }

  function createNewFile() {
    fileManager.createNewFile(keyword).then(() => {});
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
</script>

<div class="flex flex-col">
  <Input bind:value={keyword} type="text" placeholder="Search or create new writing" class="px-2 mb-8" />
  <div class="grid grid-cols-2 gap-4">
    {#if filteredFavourites().length === 0}
      <Item.Root
        variant="outline"
        size="default"
        class="flex text-muted-foreground items-center cursor-pointer hover:text-white p-3"
      >
        {#snippet child({ props })}
          <button class="unset" onclick={createNewFile} {...props}>
            <Item.Media class="">
              <FileIcon class="size-5" />
            </Item.Media>
            <Item.Content class="">
              <Item.Title class="">
                <span class="capitalize">
                  Create <span class="text-bold lowercase">{keyword}.md</span></span
                >
              </Item.Title>
            </Item.Content>
            <Item.Actions class="">
              <PlusIcon class="size-4" />
            </Item.Actions>
          </button>
        {/snippet}
      </Item.Root>
    {/if}

    {#each filteredFavourites() as item (item.path)}
      <Item.Root
        variant="outline"
        size="default"
        class="flex text-muted-foreground items-center cursor-pointer hover:text-white p-3"
      >
        {#snippet child({ props })}
          <a href={resolve(`/${item.name}`)} {...props}>
            <Item.Media class="">
              <FileIcon class="size-5" />
            </Item.Media>
            <Item.Content class="">
              <Item.Title class="">
                <span class="capitalize"> {formatFileName(item.name)}</span>
              </Item.Title>
            </Item.Content>
            <Item.Actions class="">
              <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                  <EllipsisIcon class="size-4" />
                </DropdownMenu.Trigger>
                <DropdownMenu.Content class="dark w-56 rounded-lg" side="right" align="start" portalProps={{}}>
                  <DropdownMenu.Item
                    onclick={() => handleDeleteClick(item)}
                    variant="destructive"
                    inset={false}
                    class=""
                  >
                    <Trash2Icon class="size-4" />
                    <span>Delete</span>
                  </DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Root>
            </Item.Actions>
          </a>
        {/snippet}
      </Item.Root>
    {/each}
  </div>
  <Button onclick={loadMore} class="mt-4 ml-auto" variant="outline" disabled={false}>More</Button>
</div>

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
