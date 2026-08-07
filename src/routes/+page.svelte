<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import BinderSidebar from '$lib/components/binder-sidebar.svelte';
  import SettingIcon from '@lucide/svelte/icons/settings';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import Button from '$lib/components/ui/button/button.svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  let activeBinder = $state(null);
  let timelineDocuments = $state(null);
</script>

<div class="relative flex flex-col md:flex-row w-full h-full overflow-hidden">
  <BinderSidebar bind:activeBinder />

  <div class="page-container flex-1 min-w-0 h-full flex flex-col overflow-hidden">
    <div class="flex items-center justify-between mb-4 md:mb-8">
      <h2 class="text-3xl md:text-5xl font-normal text-foreground pl-10 xl:pl-0 truncate">{activeBinder ?? 'Writings'}</h2>
      <div class="flex items-center gap-1.5 shrink-0">
        <Button
          onclick={() => timelineDocuments?.handleTreeCreateFile([])}
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-foreground rounded-full"
          aria-label="New entry"
        >
          <PlusIcon class="size-3" strokeWidth={1.5} />
        </Button>
        <Button
          onclick={()=>goto(resolve('/settings'))}
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-foreground rounded-full"
          aria-label="Settings"
        >
          <SettingIcon class="size-3.5" />
        </Button>
      </div>
    </div>
    <div class="flex-1 overflow-hidden w-full relative">
      <TimelineDocuments bind:this={timelineDocuments} {activeBinder} />
    </div>
  </div>
</div>
