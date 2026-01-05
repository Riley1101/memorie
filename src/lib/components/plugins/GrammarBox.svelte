<script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import ChevrondownIcon from '@lucide/svelte/icons/chevron-down';
  import { Label } from '$lib/components/ui/label/index.js';
  import { Textarea } from '$lib/components/ui/textarea/index.js';
  import { llmManager } from '@/runes/llm.svelte.js';

  /**
   * @typedef {Object} GrammarBoxProps
   * @property {string} originalText - The original text that needs grammar suggestions.
   * @property {(suggestion: string) => void} onFix - Callback function to apply the suggested fix.
   * @property {number} sequence - The sequence number of the text segment.
   */

  function handleSend() {}

  /** @type {GrammarBoxProps} */
  let { originalText, onFix, sequence } = $props();

  const suggestion = originalText.toUpperCase();

  const exampleSuggestions = [
    'Correct grammar',
    'Improve clarity',
    'Make it more formal',
    'Simplify the language',
  ];
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button {...props} class="not-prose flex items-center gap-0 " size="sm" variant="outline">
        <AiSparkleIcon class="size-3" />
        <span class="text-xs mx-2"> Ai Suggestions </span>
        <ChevrondownIcon class="size-4" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content class="w-100 dark" align="start" size="sm">
    <div class="flex w-full max-w-sm flex-col gap-1.5 p-2">
      <Label for="prompt" class="text-xs! mb-1">Prompt</Label>
      <div class="flex gap-2 flex-col">
        <Textarea
          type="email"
          id="prompt"
          placeholder="Ask AI"
          size="sm"
          class="text-xs!"
          variant="outline"
        />
        <Button size="sm" variant="outline" onclick={handleSend} class="max-w-max">Send</Button>
      </div>
    </div>
    <DropdownMenu.Label class="font-normal text-xs text-muted-foreground"
      >Suggestions</DropdownMenu.Label
    >
    <DropdownMenu.Group>
      {#each exampleSuggestions as example}
        <DropdownMenu.Item
          class="cursor-pointer hover:bg-accent/50 text-xs"
          on:click={() => onFix(`${example} applied to: ${originalText}`)}
        >
          {example}
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Group>
  </DropdownMenu.Content>
</DropdownMenu.Root>
