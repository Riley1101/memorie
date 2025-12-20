<script>
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import ChevrondownIcon from '@lucide/svelte/icons/chevron-down';

  let { originalText, onFix } = $props();

  const suggestion = originalText.toUpperCase();

</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button {...props} variant="outline" class="-translate-y-4 not-prose flex items-center gap-0" size="sm">
        <AiSparkleIcon class="size-3" />
        <span class="text-xs mx-2">
          4 Ai Suggestions
        </span>
        <ChevrondownIcon class="size-4" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content class="w-100 dark" align="start" size="sm">
    <DropdownMenu.Label class="font-normal text-xs px-2">Suggestions</DropdownMenu.Label>
    <DropdownMenu.Group>
      <DropdownMenu.Item class="group p-1 flex gap-0 flex-col items-start justify-start" onclick={()=> onFix(suggestion)}>
        <p class="text-xs mb-1">
          Lorem ipsum dolor sit amet.
        </p>
        <p class="text-xs text-muted-foreground">This is the reason is not working.</p>
      </DropdownMenu.Item>
    </DropdownMenu.Group>
  </DropdownMenu.Content>
</DropdownMenu.Root>
