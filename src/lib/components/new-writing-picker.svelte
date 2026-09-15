<script>
  import * as Command from '$lib/components/ui/command/index.js';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import CornerDownRightIcon from '@lucide/svelte/icons/corner-down-right';
  import { appState } from '$lib/runes/app.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { startNewWriting } from '$lib/new-writing.js';

  /**
   * Every place a writing can live: the top level, each binder, and each
   * nested folder, sorted so a binder's folders sit right under it.
   */
  let locations = $derived.by(() => {
    const dirs = new Set([...fileManager.binders, ...fileManager.folders]);
    return [...dirs].sort((a, b) => a.localeCompare(b)).map((dir) => {
      const segments = dir.split('/');
      return {
        dir,
        label: segments[segments.length - 1],
        depth: segments.length - 1,
        parent: segments.slice(0, -1).join(' / '),
      };
    });
  });

  function choose(dir) {
    appState.toggleNewPicker(false);
    startNewWriting(dir);
  }
</script>

<Command.Dialog
  open={appState.ui.isNewPickerOpen}
  onOpenChange={(v) => appState.toggleNewPicker(v)}
  title="New writing in…"
  description="Choose where the new writing should live"
  portalProps={{}}
>
  <Command.Input placeholder="New writing in…" class="border-border/20 bg-muted/10" />
  <Command.List class="p-2">
    <Command.Empty class="py-10 text-muted-foreground/60 font-sans">No such place.</Command.Empty>

    <Command.Group heading="Location" class="" value="">
      <Command.Item
        value="top level all writings"
        onSelect={() => choose('')}
        class="rounded-lg px-3 py-2.5 mb-1 aria-selected:bg-muted/40"
      >
        <FileIcon strokeWidth={1.5} class="size-4 mr-1" />
        <span class="font-writer text-base">Top level</span>
        <span class="ml-auto text-[0.6875rem] text-muted-foreground/50 font-mono">no binder</span>
      </Command.Item>
      {#each locations as loc (loc.dir)}
        <Command.Item
          value={loc.dir}
          onSelect={() => choose(loc.dir)}
          class="rounded-lg px-3 py-2.5 mb-1 last:mb-0 aria-selected:bg-muted/40"
        >
          {#if loc.depth > 0}
            <span style="width: {loc.depth * 0.875}rem" class="shrink-0"></span>
            <CornerDownRightIcon strokeWidth={1.5} class="size-3.5 mr-1 text-muted-foreground/40" />
          {:else}
            <FolderIcon strokeWidth={1.5} class="size-4 mr-1" />
          {/if}
          <span class="font-writer text-base truncate">{loc.label}</span>
          {#if loc.parent}
            <span class="ml-auto text-[0.6875rem] text-muted-foreground/50 font-mono truncate">{loc.parent}</span>
          {/if}
        </Command.Item>
      {/each}
    </Command.Group>
  </Command.List>
</Command.Dialog>
