<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import BinderSidebar from '$lib/components/binder-sidebar.svelte';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { MOD_KEY } from '$lib/keyboard.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { startNewWriting } from '$lib/new-writing.js';

  let timelineDocuments = $state(null);

  let newLabel = $derived(
    appState.ui.activeBinder ? `New in ${appState.ui.activeBinder}` : 'New writing'
  );
</script>

<div class="relative flex flex-col md:flex-row w-full h-full overflow-hidden">
  <BinderSidebar bind:activeBinder={appState.ui.activeBinder} />

  <div class="page-container flex-1 min-w-0 h-full flex flex-col overflow-hidden">
    <div class="flex items-center justify-between gap-3 mb-4 md:mb-8">
      <h2 class="text-3xl md:text-5xl font-normal text-foreground pl-[calc(max(1rem,var(--titlebar-inset-left,0px))+2.25rem)] xl:pl-0 truncate">{appState.ui.activeBinder ?? 'Writings'}</h2>
      <div class="flex items-center gap-1 shrink-0">
        <!-- Primary action: one click, no naming, lands in the open binder. -->
        <div class="flex items-center mr-1.5 rounded-full border border-border/60 bg-muted/20 hover:bg-muted/40 transition-colors">
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={() => startNewWriting(appState.ui.activeBinder ?? '')}
                  class="flex items-center gap-1.5 h-8 pl-3 pr-2.5 rounded-l-full text-[0.8125rem] font-medium text-foreground/90 hover:text-foreground"
                >
                  <PlusIcon strokeWidth={1.75} class="size-3.5" />
                  <span class="hidden sm:inline">{newLabel}</span>
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="bottom" portalProps={{}}>{newLabel} · {MOD_KEY}N</Tooltip.Content>
          </Tooltip.Root>
          <div class="w-px h-4 bg-border/60"></div>
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={() => appState.toggleNewPicker(true)}
                  aria-label="New writing in another place"
                  class="flex items-center justify-center h-8 w-7 rounded-r-full text-muted-foreground hover:text-foreground"
                >
                  <ChevronDownIcon strokeWidth={1.75} class="size-3.5" />
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="bottom" portalProps={{}}>New in… · {MOD_KEY}⇧N</Tooltip.Content>
          </Tooltip.Root>
        </div>
      </div>
    </div>
    <div class="flex-1 overflow-hidden w-full relative">
      <TimelineDocuments bind:this={timelineDocuments} activeBinder={appState.ui.activeBinder} />
    </div>
  </div>
</div>
