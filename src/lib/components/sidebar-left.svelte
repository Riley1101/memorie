<script>
  import NavFavorites from './nav-favorites.svelte';
  import NavMain from './nav-main.svelte';
  import TeamSwitcher from './team-switcher.svelte';
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  let { ref = $bindable(null), ...restProps } = $props();
</script>

<Sidebar.Root class="border-r-0" {...restProps}>
  <Sidebar.Header>
    <TeamSwitcher />
    <NavMain />
  </Sidebar.Header>
  <Sidebar.Content>
    <NavFavorites />
  </Sidebar.Content>
  <Sidebar.Rail />
</Sidebar.Root>
