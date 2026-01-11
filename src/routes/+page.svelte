<script>
  import { Separator } from '$lib/components/ui/separator/index.js';
  import AiChatStream from '$lib/components/ai-chat-stream.svelte';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import LoadingCircle from '@lucide/svelte/icons/loader-circle';
  import * as InputGroup from '$lib/components/ui/input-group';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import HomeFavourites from '$lib/components/home-favourites.svelte';
  import HomeAiChat from '@/components/home-ai-chat.svelte';
  import { llmManager } from '@/runes/llm.svelte';

  /**
   * Represents the structure of the "Thing" parent object.
   * (Update this definition based on your actual Rust struct for Thing)
   * @typedef {Object} Thing
   */

  /**
   * Represents a result returned from a search query.
   * Corresponds to the Rust struct `SearchResult`.
   *
   * @typedef {Object} SearchResult
   * @property {Thing} parent - The parent entity associated with this result.
   * @property {string} content - The main textual content found.
   * @property {number} sequence - The sequence index (usize maps to number in JS).
   * @property {string} title - The title of the result.
   */

  /** @type {string} */
  let commandInput = $state('');

  /** @type {{data: SearchResult[]}|null} */
  let result = $state(null);

  async function handleSubmit() {
    llmManager.sendRagMessage(commandInput).then((res) => {

      result = res;
    });
  }
</script>

<div class="p-4 overflow-hidden container mx-auto max-w-3xl w-full h-dvh grid grid-rows-[1fr_auto]">
  <div class="overflow-hidden">
    <ScrollArea type="scroll" class="h-full">
      {#if commandInput === ''}
        <h2 class="py-8 text-4xl">Start writing down your thoughts</h2>
        <HomeFavourites />
      {:else}
        <HomeAiChat {result} />
        <AiChatStream type="rag" />
      {/if}
    </ScrollArea>
  </div>

  <div class="flex flex-col gap-2">
    <InputGroup.Root>
      <InputGroup.Textarea
        placeholder="Ask, Search or Chat..."
        class="text-base!"
        bind:value={commandInput}
        onkeydown={(/** @type KeyboardEvent */ e) => {
          if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            handleSubmit();
          }
        }}
      />
      <InputGroup.Addon align="block-end">
        <Separator orientation="vertical" class="!h-4" />
        <InputGroup.Button
          disabled={llmManager.isRAGLoading}
          onclick={() => {
            handleSubmit();
          }}
          variant="default"
          class="ml-auto rounded-full"
          size="icon-xs"
        >
          {#if llmManager.isRAGLoading}
            <LoadingCircle class="animate-spin size-4" />
          {:else}
            <ArrowUpIcon class="size-4" />
          {/if}
          <span class="sr-only">Send</span>
        </InputGroup.Button>
      </InputGroup.Addon>
    </InputGroup.Root>
  </div>
</div>
