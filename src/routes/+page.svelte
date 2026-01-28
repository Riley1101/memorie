<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import MenuIcon from '@lucide/svelte/icons/menu';
  import { appState } from '$lib/runes/app.svelte.js';
</script>

  <div class="w-full h-full flex flex-col p-8 md:p-12 max-w-3xl mx-auto overflow-hidden">
    <div class="flex items-center justify-between mb-8">
      <h2 class="text-5xl font-normal">Documents</h2>
      <button 
        onclick={()=>appState.toggleSidebar(!appState.ui.isSidebarOpen)}
        class="opacity-20 hover:opacity-100 transition-opacity md:hidden"
      >
        <MenuIcon class="size-6" />
      </button>
    </div>
    <div class="flex-1 overflow-hidden w-full">
      <ScrollArea type="scroll" class="w-full h-full">
        <TimelineDocuments />
      </ScrollArea>
    </div>
  </div>
