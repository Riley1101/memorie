<script>
  import { resolve } from '$app/paths';
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import FileStackIcon from '@lucide/svelte/icons/file-stack';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
</script>

<Sidebar.Menu class="">
  <Sidebar.MenuItem class="">
    <Sidebar.MenuButton
      class=""
      tooltipContent="Documents"
      tooltipContentProps={{}}
      children={undefined}
    >
      {#snippet child({ props })}
        <a href={resolve('/')} {...props}>
          <FileStackIcon />
          Documents
        </a>
      {/snippet}
    </Sidebar.MenuButton>
  </Sidebar.MenuItem>
</Sidebar.Menu>
