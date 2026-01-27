<script>
  import {
    Message,
    MessageAction,
    MessageActions,
    MessageContent,
    MessageResponse,
  } from "$lib/components/ai-elements/new-message/index.js";

  import Copy from "@lucide/svelte/icons/copy";
  import RefreshCcw from "@lucide/svelte/icons/refresh-ccw";
  import { llmManager } from "@/runes/llm.svelte";
  import LoaderIcon from "@lucide/svelte/icons/loader-2";

  /**
   * @type {{ type?: "rag" | "chat" }}
   */
  let { type = "chat"} = $props();

  let messages = $derived(type === "rag" ? llmManager.ragMessages : llmManager.messages);

  function handleCopy(content) {
    if (!content) return;
    navigator.clipboard.writeText(content);
  }

  function handleRetry(index) {
    // TODO: Implement retry logic in llmManager
    console.log("Retrying message at index:", index);
  }

</script>

<div class="flex flex-col gap-6 p-4">
  {#each messages as message, index (index)}
    <Message from={message.role}>

      <MessageContent>
        {#if message.role === "assistant"}
          {#if message.content === ""}
            <div class="flex items-center gap-2 text-muted-foreground">
              <LoaderIcon class="size-4 animate-spin" />
              <span class="text-xs">Thinking...</span>
            </div>
          {:else}
            <MessageResponse content={message.content} />
          {/if}
        {:else}
          <div class="whitespace-pre-wrap">{message.content}</div>
        {/if}
      </MessageContent>

      {#if message.role === "assistant" && message.content !== ""}
        <MessageActions>
          <MessageAction
            label="Retry"
            onclick={() => handleRetry(index)}
            tooltip="Regenerate response"
          >
            <RefreshCcw class="size-3.5" />
          </MessageAction>

          <MessageAction
            label="Copy"
            onclick={() => handleCopy(message.content)}
            tooltip="Copy to clipboard"
          >
            <Copy class="size-3.5" />
          </MessageAction>
        </MessageActions>
      {/if}
    </Message>
  {/each}
</div>