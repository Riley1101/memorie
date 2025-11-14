<script>
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  let { ref = $bindable(null), ...restProps } = $props();

</script>

<Sidebar.Root
  bind:ref
  side="right"
  collapsible="icon"
  class="sticky top-0 hidden h-svh border-r lg:flex"
  {...restProps}
>
  <Sidebar.Header></Sidebar.Header>
  <Sidebar.Content></Sidebar.Content>
  <Sidebar.Rail />
</Sidebar.Root>
