<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import BinderSidebar from '$lib/components/binder-sidebar.svelte';
  import SettingIcon from '@lucide/svelte/icons/settings';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  let activeBinder = $state(null);
</script>

<div class="relative flex flex-col md:flex-row w-full h-full overflow-hidden">
  <BinderSidebar bind:activeBinder />

  <div class="page-container flex-1 min-w-0 h-full flex flex-col overflow-hidden">
    <div class="flex items-center justify-between mb-4 md:mb-8">
      <h2 class="text-3xl md:text-5xl font-normal text-foreground pl-10 xl:pl-0">Writings</h2>
      <button
        onclick={()=>goto(resolve('/settings'))}
        class="text-muted-foreground hover:text-foreground transition-colors"
      >
        <SettingIcon class="size-5" />
      </button>
    </div>
    <div class="flex-1 overflow-hidden w-full relative">
      <TimelineDocuments {activeBinder} />
    </div>
  </div>
</div>
