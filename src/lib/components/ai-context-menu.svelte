<script>
  import PlusIcon from '@lucide/svelte/icons/plus';
  import * as Command from '$lib/components/ui/command/index.js';
  import * as Popover from '$lib/components/ui/popover/index.js';
  import Button from '$lib/components/ui/button/button.svelte';

  let open = $state(false);
  let value = $state('');
  let triggerRef = $state(null);
</script>

<Popover.Root bind:open>
  <Popover.Trigger bind:ref={triggerRef}>
    {#snippet child({ props })}
      <Button
        variant="outline"
        class="w-[200px] justify-between"
        {...props}
        role="combobox"
        aria-expanded={open}
      >
        <PlusIcon />
      </Button>
    {/snippet}
  </Popover.Trigger>
  <Popover.Content class="dark w-[200px] p-0" side="top" align="start" sideOffset={8}>
    <Command.Root>
      <Command.Input placeholder="Search context..." bind:value />
      <Command.List>
        <Command.Empty>No context found.</Command.Empty>
        <Command.Group></Command.Group>
      </Command.List>
    </Command.Root>
  </Popover.Content>
</Popover.Root>
