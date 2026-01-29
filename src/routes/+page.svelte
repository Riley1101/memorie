<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import SettingIcon from '@lucide/svelte/icons/settings';
  import { goto } from '$app/navigation';
  import { llmManager } from '$lib/runes/llm.svelte.js';
</script>

  <div class="w-full h-full flex flex-col p-8 md:p-12 max-w-3xl mx-auto overflow-hidden">
    <div class="flex items-center justify-between mb-8">
      <h2 class="text-5xl font-normal text-foreground">Documents</h2>
      <button 
        onclick={()=>goto('/settings')}
        class="opacity-20 hover:opacity-100 transition-opacity"
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
