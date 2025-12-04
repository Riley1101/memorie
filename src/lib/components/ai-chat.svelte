<script>
  import * as InputGroup from '$lib/components/ui/input-group/index.js';
  import { Separator } from '$lib/components/ui/separator/index.js';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import PauseIcon from '@lucide/svelte/icons/pause';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte.js';
  import Button from '$lib/components/ui/button/button.svelte';
  import AiContextMenu from './ai-context-menu.svelte';

  let context = $derived(memoryManager.context);

  let prompt = $state('');

  async function handleSubmit() {
    if (context) {
      prompt = `${prompt}`;
    }
    await llmManager.sendMessage(prompt);
    prompt = '';
  }
</script>

<div class="flex flex-col gap-2">
  <Button class="max-w-max" size="sm" variant="outline" onclick={() => llmManager.newSession()}>
    <PlusIcon />
    New Session
  </Button>
  <InputGroup.Root>
    <InputGroup.Textarea placeholder="Ask, Search or Chat..." bind:value={prompt} />
    <InputGroup.Addon align="block-end">
      <InputGroup.Button
        variant="outline"
        class="rounded-full"
        size="icon-xs"
        aria-label="Add context"
      >
        <AiContextMenu />
      </InputGroup.Button>
      <Separator orientation="vertical" class="!h-4" />
      <InputGroup.Button
        variant="default"
        class="ml-auto rounded-full"
        size="icon-xs"
        disabled={!llmManager.modelsLoaded}
        onclick={() => {
          if (llmManager.isLoading) {
            llmManager.cancelMessage();
            return;
          }
          handleSubmit();
        }}
      >
        {#if llmManager.isLoading}
          <PauseIcon />
        {:else}
          <ArrowUpIcon />
        {/if}
        <span class="sr-only">Send</span>
      </InputGroup.Button>
    </InputGroup.Addon>
  </InputGroup.Root>
</div>
