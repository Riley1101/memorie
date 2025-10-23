<script>
  import * as InputGroup from '$lib/components/ui/input-group/index.js';
  import { Separator } from '$lib/components/ui/separator/index.js';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte.js';

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

<InputGroup.Root>
  <InputGroup.Textarea placeholder="Ask, Search or Chat..." bind:value={prompt} />
  <InputGroup.Addon align="block-end">
    <InputGroup.Button
      onclick={() => llmManager.newSession()}
      variant="outline"
      class="rounded-full"
      size="icon-xs"
    >
      <PlusIcon />
    </InputGroup.Button>
    <Separator orientation="vertical" class="!h-4" />
    <InputGroup.Button
      variant="default"
      class="ml-auto rounded-full"
      size="icon-xs"
      onclick={handleSubmit}
    >
      <ArrowUpIcon />
      <span class="sr-only">Send</span>
    </InputGroup.Button>
  </InputGroup.Addon>
</InputGroup.Root>
