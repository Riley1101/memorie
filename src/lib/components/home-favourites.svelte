<script>
  import * as Item from '$lib/components/ui/item/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { fileManager } from '@/runes/fs.svelte';
  import { resolve } from '$app/paths';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import FileIcon from '@lucide/svelte/icons/file';
  import Button from './ui/button/button.svelte';

  let favourites = $derived(fileManager.files);

  let limit = $state(5);

  let filteredFavourites = $derived(() => {
    return favourites.slice(0, limit);
  });

  function loadMore() {
    limit += 4;
  }
</script>

<div class="flex flex-col">
  <div class="grid grid-cols-2 gap-4">
    {#each filteredFavourites() as item (item.path)}
      <Item.Root
        variant="outline"
        class="flex text-muted-foreground items-center cursor-pointer hover:text-white p-3"
      >
        {#snippet child({ props })}
          <a href={resolve(`/${item.name}`)} {...props}>
            <Item.Media>
              <FileIcon class="size-5" />
            </Item.Media>
            <Item.Content>
              <Item.Title>{item.name}</Item.Title>
            </Item.Content>
            <Item.Actions>
              <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                  <EllipsisIcon class="size-4" />
                </DropdownMenu.Trigger>
                <DropdownMenu.Content class="dark w-56 rounded-lg" side="right" align="start">
                  <DropdownMenu.Item>
                    <Trash2Icon class="text-muted-foreground" />
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
  <Button onclick={loadMore} class="mt-4 ml-auto" variant="outline">More</Button>
</div>
