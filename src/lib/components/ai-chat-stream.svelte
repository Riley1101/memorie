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
  import FileText from "@lucide/svelte/icons/file-text";

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
    <Message from={message.role} class="">

      <MessageContent class="">
        {#if message.role === "assistant"}
          {#if message.content === ""}
            <div class="flex items-center gap-2 text-muted-foreground">
              <LoaderIcon class="size-4 animate-spin" />
              <span class="text-xs">Thinking...</span>
            </div>
          {:else}
            <MessageResponse content={message.content} class="" />
            {#if message.references && message.references.length > 0}
              <div class="mt-4 flex flex-wrap gap-2 border-t border-border/40 pt-4">
                <div class="w-full text-[10px] uppercase font-bold text-muted-foreground/60 tracking-wider mb-1">
                  Sources
                </div>
                {#each message.references as ref}
                  <div class="flex items-center gap-1.5 px-3 py-1 rounded-md bg-secondary/20 text-xs text-muted-foreground border border-border/40 hover:bg-secondary/40 transition-colors cursor-default">
                    <FileText class="size-3.5 opacity-60" />
                    <span>{ref.title}</span>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}
        {:else}
          <div class="whitespace-pre-wrap">{message.content}</div>
        {/if}
      </MessageContent>

      {#if message.role === "assistant" && message.content !== ""}
        <MessageActions class="">
          <MessageAction
            label="Retry"
            onclick={() => handleRetry(index)}
            tooltip="Regenerate response"
            class=""
          >
            <RefreshCcw class="size-3.5" />
          </MessageAction>

          <MessageAction
            label="Copy"
            onclick={() => handleCopy(message.content)}
            tooltip="Copy to clipboard"
            class=""
          >
            <Copy class="size-3.5" />
          </MessageAction>
        </MessageActions>
      {/if}
    </Message>
  {/each}
</div>