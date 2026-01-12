<script>
  import {
    PromptInput,
    PromptInputBody,
    PromptInputSubmit,
    PromptInputTextarea,
    PromptInputToolbar,
    PromptInputTools,
    PromptInputAttachments,
    PromptInputAttachment,
    PromptInputActionMenu,
    PromptInputActionMenuTrigger,
    PromptInputActionMenuContent,
    PromptInputActionAddAttachments,
  } from "$lib/components/ai-elements/prompt-input/index.js";
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte.js';

  /** @type {{type ?: "chat" | "rag"}} */
  let { type = "chat"} = $props();

  let text = $state('');
  let context = $derived(memoryManager.context);

  /**
   * Handles the submission from the PromptInput component.
   * @param {{ text: string, files: File[] }} message
   */
  async function handleSubmit(message) {
    const rawText = message.text;
    const attachments = message.files;
    if (!rawText && (!attachments || attachments.length === 0)) return;
    let finalPrompt = rawText || '';
    if (context) {
      finalPrompt = `${finalPrompt}`;
    }
    text = '';
    if (type === 'rag') {
      await llmManager.sendRagMessage(finalPrompt);
      return;
    }
    await llmManager.sendMessage(finalPrompt);
  }

  function handleStop() {
    llmManager.cancelMessage();
  }

  function handleNewSession() {
    if (type === 'rag') {
      llmManager.newRagSession();
      return;
    }
    llmManager.newSession();
  }
</script>

<div class="flex flex-col gap-2 dark">
  <PromptInput
    onSubmit={handleSubmit}
    onStop={handleStop}
    class="relative w-full"
    globalDrop
    multiple
  >
    <PromptInputBody>
      <PromptInputAttachments>
        {#snippet children(attachment)}
          <PromptInputAttachment data={attachment} />
        {/snippet}
      </PromptInputAttachments>

      <PromptInputTextarea
        placeholder="Ask, Search or Chat..."
        bind:value={text}
        onchange={(e) => (text = e.target.value)}
      />
    </PromptInputBody>

    <PromptInputToolbar>
      <PromptInputTools>

        <PromptInputActionMenu>
          <PromptInputActionMenuTrigger />
          <PromptInputActionMenuContent>
            <PromptInputActionAddAttachments />
            <button
              class="relative flex w-full cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-accent hover:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
              onclick={handleNewSession}
            >
              <PlusIcon class="size-4" />
              <span>New Session</span>
            </button>
          </PromptInputActionMenuContent>
        </PromptInputActionMenu>


      </PromptInputTools>

      <PromptInputSubmit
        status={llmManager.isLoading ? "streaming" : "ready"}
      />
    </PromptInputToolbar>
  </PromptInput>
</div>