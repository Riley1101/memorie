<script>
  import * as Command from '$lib/components/ui/command/index.js';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import CornerDownRightIcon from '@lucide/svelte/icons/corner-down-right';
  import { fileManager } from '$lib/runes/fs.svelte.js';

  /**
   * @type {{
   *   open: boolean,
   *   title?: string,
   *   subject?: string,
   *   currentDir?: string,
   *   excludePrefix?: string | null,
   *   allowTopLevel?: boolean,
   *   onChoose: (dir: string) => void,
   * }}
   * `excludePrefix` hides that folder and everything under it (a folder can't
   * move into itself). `currentDir` is shown but marked as where it already is.
   */
  let {
    open = $bindable(false),
    title = 'Move to…',
    subject = '',
    currentDir = '',
    excludePrefix = null,
    allowTopLevel = true,
    onChoose,
  } = $props();

  let locations = $derived.by(() => {
    const dirs = new Set([...fileManager.binders, ...fileManager.folders]);
    return [...dirs]
      .filter((dir) => !excludePrefix || (dir !== excludePrefix && !dir.startsWith(`${excludePrefix}/`)))
      .sort((a, b) => a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' }))
      .map((dir) => {
        const segments = dir.split('/');
        return {
          dir,
          label: segments[segments.length - 1],
          depth: segments.length - 1,
          parent: segments.slice(0, -1).join(' / '),
          isCurrent: dir === currentDir,
        };
      });
  });

  function choose(dir) {
    open = false;
    onChoose(dir);
  }
</script>

<Command.Dialog bind:open {title} description="Choose a destination" portalProps={{}}>
  <Command.Input placeholder={subject ? `Move “${subject}” to…` : 'Move to…'} class="border-border/20 bg-muted/10" />
  <Command.List class="p-2">
    <Command.Empty class="py-10 text-muted-foreground/60 font-sans">No such place.</Command.Empty>

    <Command.Group heading="Destination" class="" value="">
      {#if allowTopLevel}
        <Command.Item
          value="top level all writings"
          onSelect={() => choose('')}
          disabled={currentDir === ''}
          class="rounded-lg px-3 py-2.5 mb-1 aria-selected:bg-muted/40"
        >
          <FileIcon strokeWidth={1.5} class="size-4 mr-1" />
          <span class="font-writer text-base">Top level</span>
          <span class="ml-auto text-[0.6875rem] text-muted-foreground/50 font-mono">
            {currentDir === '' ? 'already here' : 'no binder'}
          </span>
        </Command.Item>
      {/if}
      {#each locations as loc (loc.dir)}
        <Command.Item
          value={loc.dir}
          onSelect={() => choose(loc.dir)}
          disabled={loc.isCurrent}
          class="rounded-lg px-3 py-2.5 mb-1 last:mb-0 aria-selected:bg-muted/40"
        >
          {#if loc.depth > 0}
            <span style="width: {loc.depth * 0.875}rem" class="shrink-0"></span>
            <CornerDownRightIcon strokeWidth={1.5} class="size-3.5 mr-1 text-muted-foreground/40" />
          {:else}
            <FolderIcon strokeWidth={1.5} class="size-4 mr-1" />
          {/if}
          <span class="font-writer text-base truncate">{loc.label}</span>
          <span class="ml-auto text-[0.6875rem] text-muted-foreground/50 font-mono truncate">
            {loc.isCurrent ? 'already here' : loc.parent}
          </span>
        </Command.Item>
      {/each}
    </Command.Group>
  </Command.List>
</Command.Dialog>
