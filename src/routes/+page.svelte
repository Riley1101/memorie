<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import SettingIcon from '@lucide/svelte/icons/settings';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
</script>

  <div class="page-container w-full h-full flex flex-col overflow-hidden">
    <div class="flex items-center justify-between mb-8">
      <h2 class="text-5xl font-normal text-foreground">Documents</h2>
      <button 
        onclick={()=>goto(resolve('/settings'))}
        class="text-muted-foreground hover:text-foreground transition-colors"
      >
        <SettingIcon class="size-5" />
      </button>
    </div>
    <div class="flex-1 overflow-hidden w-full relative">
      <ScrollArea type="scroll" class="w-full h-full">
        <TimelineDocuments />
      </ScrollArea>
    </div>
  </div>
