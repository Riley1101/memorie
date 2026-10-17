<script>
  import TimelineDocuments from '$lib/components/timeline-documents.svelte';
  import BinderSidebar from '$lib/components/binder-sidebar.svelte';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { MOD_KEY } from '$lib/keyboard.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { startNewWriting } from '$lib/new-writing.js';
  import { fileManager, baseOf } from '$lib/runes/fs.svelte.js';
  import { formatFileName } from '$lib/utils.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import FileOutputIcon from '@lucide/svelte/icons/file-output';
  import CornerDownLeftIcon from '@lucide/svelte/icons/corner-down-left';

  let newLabel = $derived(
    appState.ui.activeBinder ? `New in ${appState.ui.activeBinder}` : 'New writing'
  );

  /**
   * The writing to offer a way back into. Only shown once the listing is in
   * and the file is still there, so a deleted or renamed writing never leaves
   * a dead button on the home screen.
   */
  let resumable = $derived.by(() => {
    const last = appState.ui.lastWriting;
    if (!last || !fileManager.hasLoadedFiles) return null;
    return fileManager.files.some((f) => f.name === last) ? last : null;
  });

  let resumeLabel = $derived(resumable ? formatFileName(baseOf(resumable)) : '');

  function resume() {
    if (resumable) goto(resolve(`/${encodeURIComponent(resumable)}`));
  }
</script>

<div class="relative flex flex-col md:flex-row w-full h-full overflow-hidden">
  <BinderSidebar bind:activeBinder={appState.ui.activeBinder} />

  <div class="page-container flex-1 min-w-0 h-full flex flex-col overflow-hidden">
    <div class="flex items-center justify-between gap-3 mb-4 md:mb-8">
      <h2 class="font-writer text-3xl md:text-5xl font-normal text-heading-foreground leading-normal pb-1 truncate">{appState.ui.activeBinder ?? 'Writings'}</h2>
      <div class="flex items-center gap-1 shrink-0">
        <!-- Back to whatever was open last: the first thing most writers want. -->
        {#if resumable}
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={resume}
                  class="hidden md:flex items-center gap-1.5 h-8 px-3 mr-1 rounded-full text-[0.8125rem] text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
                >
                  <CornerDownLeftIcon strokeWidth={1.75} class="size-3.5" />
                  <span class="max-w-[14ch] truncate">{resumeLabel}</span>
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="bottom" portalProps={{}}>Carry on with “{resumeLabel}”</Tooltip.Content>
          </Tooltip.Root>
        {/if}

        <!-- Export is a headline feature; it was reachable only from the palette. -->
        {#if appState.ui.activeBinder}
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={() => appState.openExport({ fileName: null, dir: appState.ui.activeBinder ?? '' })}
                  aria-label="Export {appState.ui.activeBinder}"
                  class="flex items-center justify-center size-8 mr-1 rounded-full text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
                >
                  <FileOutputIcon strokeWidth={1.75} class="size-3.5" />
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="bottom" portalProps={{}}>Export {appState.ui.activeBinder}…</Tooltip.Content>
          </Tooltip.Root>
        {/if}

        <!-- Primary action: one click, no naming, lands in the open binder. -->
        <div class="flex items-center mr-1.5 rounded-full bg-primary hover:bg-primary-hover transition-colors">
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={() => startNewWriting(appState.ui.activeBinder ?? '')}
                  class="flex items-center gap-1.5 h-8 pl-3 pr-2.5 rounded-l-full text-[0.8125rem] font-medium text-primary-foreground"
                >
                  <PlusIcon strokeWidth={1.75} class="size-3.5" />
                  <span class="hidden sm:inline">{newLabel}</span>
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="bottom" portalProps={{}}>{newLabel} · {MOD_KEY}N</Tooltip.Content>
          </Tooltip.Root>
          <div class="w-px h-4 bg-primary-foreground/20"></div>
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  onclick={() => appState.toggleNewPicker(true)}
                  aria-label="New writing in another place"
                  class="flex items-center justify-center h-8 w-7 rounded-r-full text-primary-foreground/70 hover:text-primary-foreground"
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
      <TimelineDocuments activeBinder={appState.ui.activeBinder} />
    </div>
  </div>
</div>
