<script>
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import { resolve } from '$app/paths';

  let { ref = $bindable(null), items, ...restProps } = $props();
</script>

<Sidebar.Group bind:ref {...restProps}>
  <Sidebar.GroupContent>
    <Sidebar.Menu>
      {#each items as item (item.title)}
        <Sidebar.MenuItem>
          <Sidebar.MenuButton>
            {#snippet child({ props })}
              <a href={resolve(item.url)} {...props}>
                <item.icon />
                <span>{item.title}</span>
              </a>
            {/snippet}
          </Sidebar.MenuButton>
          {#if item.badge}
            <Sidebar.MenuBadge>{item.badge}</Sidebar.MenuBadge>
          {/if}
        </Sidebar.MenuItem>
      {/each}
    </Sidebar.Menu>
  </Sidebar.GroupContent>
</Sidebar.Group>
