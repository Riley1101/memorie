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

  /**
   * @type {  Array<{ name: string, path: string, emoji?: string }> }
   */
  let favourites = $derived(fileManager.files);

  const sidebar = useSidebar();
</script>

<Sidebar.Group class="group-data-[collapsible=icon]:hidden">
  <Sidebar.GroupLabel>Recents</Sidebar.GroupLabel>
  <Sidebar.Menu>
    {#each favourites as item (item.name)}
      <Sidebar.MenuItem>
        <Sidebar.MenuButton>
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
          >
            <DropdownMenu.Item>
              <Trash2Icon class="text-muted-foreground" />
              <span>Delete</span>
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </Sidebar.MenuItem>
    {/each}
  </Sidebar.Menu>
</Sidebar.Group>
