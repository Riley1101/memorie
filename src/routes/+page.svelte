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
  import AiChat from '@/components/ai-chat.svelte';

  /** @type {{data: any[]}|null} */
  let result = $state(null);
</script>

<div class="p-4 overflow-hidden container max-w-3xl mx-auto w-full h-dvh grid grid-rows-[1fr_auto]">
  <div class="overflow-hidden ">
    <ScrollArea type="scroll" class="h-full">
      {#if llmManager.ragMessages.length === 0 && !llmManager.isRAGLoading}
        <h2 class="py-8 text-4xl">Start writing down your thoughts</h2>
        <HomeFavourites />
      {:else}
        <HomeAiChat {result} />
        <AiChatStream type="rag" />
      {/if}
    </ScrollArea>
  </div>
<AiChat type="rag"/>
  <!--
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
  -->

</div>
