<script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import { useSidebar } from '$lib/components/ui/sidebar/index.js';
  import FileTextIcon from '@lucide/svelte/icons/file';
  import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
  import ArrowUpRightIcon from '@lucide/svelte/icons/arrow-up-right';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import { formatFileName } from '$lib/utils.js';
  import { resolve } from '$app/paths';

  /**
   * @type {{ favourites: Array<{ name: string, path: string, emoji?: string }> }}
   */
  let { favourites } = $props();

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
              <ArrowUpRightIcon class="text-muted-foreground" />
              <span>Open in New Tab</span>
            </DropdownMenu.Item>
            <DropdownMenu.Separator />
            <DropdownMenu.Item>
              <Trash2Icon class="text-muted-foreground" />
              <span>Delete</span>
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </Sidebar.MenuItem>
    {/each}
    <Sidebar.MenuItem>
      <Sidebar.MenuButton class="text-sidebar-foreground/70">
        <EllipsisIcon />
        <span>More</span>
      </Sidebar.MenuButton>
    </Sidebar.MenuItem>
  </Sidebar.Menu>
</Sidebar.Group>
