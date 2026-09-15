<script>
  import {
    PromptInput,
    PromptInputBody,
    PromptInputButton,
    PromptInputSubmit,
    PromptInputTextarea,
    PromptInputToolbar,
    PromptInputTools,
  } from '$lib/components/ai-elements/prompt-input/index.js';
  import SquarePenIcon from '@lucide/svelte/icons/square-pen';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte.js';

  let text = $state('');

  /**
   * Handles the submission from the PromptInput component. The open document always
   * goes along as optional context; the backend decides whether the message needs it,
   * needs a notes search instead, or neither.
   * @param {{ text: string, files: File[] }} message
   */
  async function handleSubmit(message) {
    const rawText = message.text?.trim();
    if (!rawText) return;

    text = '';
    await llmManager.sendMessage(rawText, memoryManager.document);
  }

  function handleStop() {
    llmManager.cancelMessage();
  }

  function handleNewSession() {
    llmManager.newSession();
  }
</script>

<div class="flex flex-col gap-2">
  <PromptInput
    onSubmit={handleSubmit}
    onStop={handleStop}
    class="relative w-full"
    allowAttachments={false}
  >
    <PromptInputBody>
      <PromptInputTextarea
        placeholder={memoryManager.document ? 'Ask about this document or search your notes…' : 'Search your notes or ask anything…'}
        bind:value={text}
        onchange={(e) => (text = e.target.value)}
 />
    </PromptInputBody>

    <PromptInputToolbar>
      <PromptInputTools>
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <PromptInputButton
                {...props}
                class="size-8 hover:text-foreground"
                aria-label="New chat"
                onclick={handleNewSession}
                disabled={llmManager.isLoading || llmManager.messages.length === 0}
              >
                <SquarePenIcon strokeWidth={1.5} class="size-3.5" />
              </PromptInputButton>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}>New chat</Tooltip.Content>
        </Tooltip.Root>
      </PromptInputTools>

      <PromptInputSubmit
        status={llmManager.isLoading ? 'streaming' : 'ready'}
        onclick={handleStop}
 />
    </PromptInputToolbar>
  </PromptInput>
</div>
