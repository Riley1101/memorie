<script>
  import HomeFavourites from '$lib/components/home-favourites.svelte';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import MenuIcon from '@lucide/svelte/icons/menu';
  import { appState } from '$lib/runes/app.svelte.js';
</script>

<div class="h-dvh flex flex-col">
  <div class="writing-surface flex-1 overflow-hidden flex flex-col">
    <div class="flex items-center justify-between mb-12">
      <h2 class="text-5xl font-normal">Documents</h2>
      <button 
        onclick={()=>appState.toggleSidebar(!appState.ui.isSidebarOpen)}
        class="opacity-20 hover:opacity-100 transition-opacity"
      >
        <MenuIcon class="size-4" />
      </button>
    </div>
    <div class="flex-1 overflow-hidden">
      <ScrollArea type="scroll" class="h-full pr-4">
        <HomeFavourites limit={50} />
      </ScrollArea>
    </div>
  </div>
</div>
