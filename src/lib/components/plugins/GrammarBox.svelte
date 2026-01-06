<script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import ChevrondownIcon from '@lucide/svelte/icons/chevron-down';
  import { Label } from '$lib/components/ui/label/index.js';
  import { Textarea } from '$lib/components/ui/textarea/index.js';
  import { llmManager } from '@/runes/llm.svelte.js';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  /**
   * @typedef {Object} GrammarBoxProps
   * @property {string} originalText - The original text that needs grammar suggestions.
   * @property {(suggestion: string) => void} onFix - Callback function to apply the suggested fix.
   * @property {number} sequence - The sequence number of the text segment.
   */

  /** @type {GrammarBoxProps} */
  let { originalText, onFix } = $props();

  let customPrompt = $state('');

  function handleAccept() {
    if (llmManager.editActionContent) {
      onFix(llmManager.editActionContent);
      llmManager.newEditActionSession();
    }
  }

  /**
   * Handle sending the prompt to the LLM manager.
   * @param {string} command - The command or prompt to send to the LLM.
   */
  function handleSend(command) {
    let text = originalText;
    if (command === 'PromptExpansion') {
      text = `PROMPT: ${customPrompt} , Input Sentence: ${originalText}`;
    }
    llmManager.sendEditActionMessage(text, command);
  }

  const exampleSuggestions = [
    {
      label: 'Correct grammar',
      command: 'CorrectGrammar',
    },
    {
      label: 'Improve clarity',
      command: 'ImproveClarity',
    },
    {
      label: 'Make it more formal',
      command: 'MakeFormal',
    },
    {
      label: 'Simplify the language',
      command: 'Simplify',
    },
  ];
</script>

{#if llmManager.editActionContent || llmManager.editActionInProgress}
  <div class="border rounded-md p-2 mb-2">
    <p>
      {#if llmManager.editActionInProgress}
        <SparklesIcon class="animate-pulse size-4 mr-2 inline" />
      {/if}
      {llmManager.editActionContent}
    </p>
    <Button onclick={handleAccept} variant="outline" size="sm">Accept</Button>
  </div>
{/if}
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
      <div class="flex gap-2 flex-col">
        <Textarea
          type="email"
          id="prompt"
          placeholder="Ask AI"
          size="sm"
          class="text-xs!"
          variant="outline"
          bind:value={customPrompt}
        />
        <Button
          size="sm"
          variant="outline"
          onclick={() => handleSend('PromptExpansion')}
          class="max-w-max">Send</Button
        >
      </div>
    </div>
    <DropdownMenu.Label class="font-normal text-xs text-muted-foreground"
      >Suggestions</DropdownMenu.Label
    >
    <DropdownMenu.Group>
      {#each exampleSuggestions as example (example.command)}
        <DropdownMenu.Item
          closeOnSelect={false}
          class="cursor-pointer hover:bg-accent/50 text-xs"
          onclick={() => handleSend(example.command)}
        >
          {example.label}
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Group>
  </DropdownMenu.Content>
</DropdownMenu.Root>
