<script>
  import { resolve } from '$app/paths';
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import HouseIcon from '@lucide/svelte/icons/house';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import { appState } from '$lib/runes/app.svelte.js';

  $inspect(`Toggling AI chat interface ${appState.isAiChatOpen ? 'closed' : 'open'}`);
</script>

<Sidebar.Menu>
  <Sidebar.MenuItem>
    <Sidebar.MenuButton>
      {#snippet child({ props })}
        <a href={resolve('/')} {...props}>
          <HouseIcon />
          <span>Home</span>
        </a>
      {/snippet}
    </Sidebar.MenuButton>
  </Sidebar.MenuItem>
  <Sidebar.MenuItem>
    <Sidebar.MenuButton onclick={appState.toggleAiChat}>
      <SparklesIcon />
      AI Search
    </Sidebar.MenuButton>
  </Sidebar.MenuItem>
</Sidebar.Menu>
