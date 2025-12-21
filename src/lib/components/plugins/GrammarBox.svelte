<script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import ChevrondownIcon from '@lucide/svelte/icons/chevron-down';
  import { editorState } from '$lib/runes/editor.svelte';

  /**
   * @typedef {Object} GrammarBoxProps
   * @property {string} originalText - The original text that needs grammar suggestions.
   * @property {(suggestion: string) => void} onFix - Callback function to apply the suggested fix.
   * @property {number} sequence - The sequence number of the text segment.
   */
  let { originalText, onFix, sequence } = $props();

  const suggestion = originalText.toUpperCase();
</script>

<code class="text-xs">
  {JSON.stringify({ sequence })}
</code>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button {...props} class="not-prose flex items-center gap-0 " size="sm" variant="outline">
        <AiSparkleIcon class="size-3" />
        <span class="text-xs mx-2"> 4 Ai Suggestions </span>
        <ChevrondownIcon class="size-4" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content class="w-100 dark" align="start" size="sm">
    <DropdownMenu.Label class="font-normal text-xs px-2">Suggestions</DropdownMenu.Label>
    <DropdownMenu.Group>
      <DropdownMenu.Item
        class="group p-1 flex gap-0 flex-col items-start justify-start"
        onclick={() => onFix(suggestion)}
      >
        <p class="text-[13px] mb-1">Lorem ipsum dolor sit amet.</p>
        <p class="text-[10px] text-muted-foreground">This is the reason is not working.</p>
      </DropdownMenu.Item>
    </DropdownMenu.Group>
  </DropdownMenu.Content>
</DropdownMenu.Root>
