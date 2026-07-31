<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import SettingIcon from '@lucide/svelte/icons/settings';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
</script>

  <div class="page-container w-full h-full flex flex-col overflow-hidden">
    <div class="flex items-center justify-between mb-4 md:mb-8">
      <h2 class="text-3xl md:text-5xl font-normal text-foreground">Writings</h2>
      <button
        onclick={()=>goto(resolve('/settings'))}
        class="text-muted-foreground hover:text-foreground transition-colors"
      >
        <SettingIcon class="size-5" />
      </button>
    </div>
    <div class="flex-1 overflow-hidden w-full relative">
      <TimelineDocuments />
    </div>
  </div>
