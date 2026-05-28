<script>
  import CirclePlusIcon from '@lucide/svelte/icons/circle-plus';
  import FileTextIcon from '@lucide/svelte/icons/file-text';
  import * as Command from '$lib/components/ui/command/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
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
</script>

<Command.Dialog 
  open={appState.ui.isCommandMenuOpen}
  onOpenChange={(v) => appState.toggleCommandMenu(v)}
  title="Search Documents"
  description="Search documents by title or perform actions"
  portalProps={{}}
>
  <Command.Input placeholder="Search documents..." class="" />
  <Command.List class="">
    <Command.Empty class="">No results found.</Command.Empty>
    
    <Command.Group heading="Documents" class="" value="">
      {#each fileManager.files as file (file.name)}
        <Command.Item onSelect={() => handleSelect(file.name)} class="">
          <FileTextIcon class="size-4 mr-2" />
          <span>{file.name}</span>
        </Command.Item>
      {/each}
    </Command.Group>

    <Command.Group heading="Actions" class="" value="">
      <Command.Item onSelect={handleCreateNew} class="">
        <CirclePlusIcon class="size-4 mr-2" />
        <span>Create a new writing</span>
        <Command.Shortcut class="">⌘N</Command.Shortcut>
      </Command.Item>
    </Command.Group>
  </Command.List>
</Command.Dialog>
