<script>
  import CirclePlusIcon from '@lucide/svelte/icons/circle-plus';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import * as Command from '$lib/components/ui/command/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { formatFileName } from '@/utils';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  function handleSelect(fileName) {
    appState.toggleCommandMenu(false);
    goto(resolve(`/${encodeURIComponent(fileName)}`));
  }

  function handleCreateNew() {
    appState.toggleCommandMenu(false);
    // Logic for creating new writing could go here, for now just placeholder
    console.log("Create new writing triggered");
  }

  function formatDate(timestamp) {
    return new Date(timestamp * 1000).toLocaleDateString([], { month: 'short', day: 'numeric' });
  }

  function formatTime(timestamp) {
    return new Date(timestamp * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
</script>

<Command.Dialog
  open={appState.ui.isCommandMenuOpen}
  onOpenChange={(v) => appState.toggleCommandMenu(v)}
  title="Search Writings"
  description="Search writings by title or perform actions"
  portalProps={{}}
>
  <Command.Input
    placeholder="Search writings..."
    class="border-border/20 bg-muted/10"
  />
  <Command.List class="p-2">
    <Command.Empty class="py-10 text-muted-foreground/60 font-sans"
      >No results found.</Command.Empty
    >

    <Command.Group heading="Writings" class="" value="">
      {#each fileManager.files as file (file.name)}
        <Command.Item
            onSelect={() => handleSelect(file.name)}
            class="rounded-lg px-3 py-2.5 mb-1 last:mb-0 aria-selected:bg-muted/40"
        >
          <div class="flex flex-col gap-0.5 min-w-0 font-writer">
            <span class="text-base font-normal truncate">{formatFileName(file.name.split('/').pop())}</span>
            <div class="flex items-center gap-2 text-[0.6875rem] text-muted-foreground/60 font-mono">
              {#if file.folder}
                <span class="flex items-center gap-1">
                  <FolderIcon class="size-2.5" />
                  {file.folder}
                </span>
                <span class="opacity-30">•</span>
              {/if}
              <span>{formatDate(file.last_modified)}</span>
              <span class="opacity-30">•</span>
              <span>{formatTime(file.last_modified)}</span>
            </div>
          </div>
        </Command.Item>
      {/each}
    </Command.Group>

    <Command.Group heading="Actions" class="" value="">
      <Command.Item onSelect={handleCreateNew} class="rounded-lg px-3 py-2.5 aria-selected:bg-muted/40">
        <CirclePlusIcon class="size-3.5 mr-2" />
        <span>Create a new writing</span>
        <Command.Shortcut class="">⌘N</Command.Shortcut>
      </Command.Item>
    </Command.Group>
  </Command.List>
</Command.Dialog>
