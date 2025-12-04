<script>
  import CirclePlusIcon from "@lucide/svelte/icons/circle-plus";
  import SearchIcon from "@lucide/svelte/icons/search";
  import * as Command from "$lib/components/ui/command/index.js";
</script>

<Command.Root class=" border shadow-md md:min-w-[450px]">
  <Command.Input placeholder="Enter command"/>
  <Command.List>
    <Command.Empty class="p-0">
      <Command.Item>
        <CirclePlusIcon class="size-4" />
        <span>Create a new writing</span>
        <Command.Shortcut>⌘N</Command.Shortcut>
      </Command.Item>
    </Command.Empty>

    <Command.Group heading="Suggestions">
      <Command.Item>
        <CirclePlusIcon class="size-4" />
        <span>Create a new writing</span>
        <Command.Shortcut>⌘N</Command.Shortcut>
      </Command.Item>
      <Command.Item>
        <SearchIcon />
        <span>Search</span>
        <Command.Shortcut>⌘S</Command.Shortcut>
      </Command.Item>
    </Command.Group>
  </Command.List>
</Command.Root>

