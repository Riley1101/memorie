  <script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import CommandIcon from '@lucide/svelte/icons/command';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import { resolve } from '$app/paths';
  import { goto } from '$app/navigation';

  let team = {
    name: 'Memorie',
    logo: CommandIcon,
    plan: 'Just You',
  };
</script>

<Sidebar.Menu class="">
  <Sidebar.MenuItem class="">
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        {#snippet child({ props })}
          <Sidebar.MenuButton class="w-fit px-1.5" {...props}>
            <div
              class="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-5 items-center justify-center rounded-md"
            >
              <team.logo class="size-3" />
            </div>
            <span class="truncate font-medium text-lg">{team.name}</span>
          </Sidebar.MenuButton>
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content class="w-56" side="bottom" align="start" portalProps={{}}>
        <DropdownMenu.Item
          class=""
          inset={false}
          onclick={() => goto(resolve('/settings'))}
        >
          <SettingsIcon class="size-4" />
          <span>Settings</span>
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  </Sidebar.MenuItem>
</Sidebar.Menu>
