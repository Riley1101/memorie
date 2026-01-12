<script>
  import { resolve } from '$app/paths';
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import FileStackIcon from '@lucide/svelte/icons/file-stack';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
</script>

<Sidebar.Menu>
  <Sidebar.MenuItem>
    <Sidebar.MenuButton>
      {#snippet child({ props })}
        <a href={resolve('/')} {...props}>
          <SparklesIcon />
          AI Search
        </a>
      {/snippet}
    </Sidebar.MenuButton>
    <Sidebar.MenuButton>
      {#snippet child({ props })}
        <a href={resolve('/documents')} {...props}>
          <FileStackIcon />
          <span>Documents</span>
        </a>
      {/snippet}
    </Sidebar.MenuButton>
  </Sidebar.MenuItem>
</Sidebar.Menu>
