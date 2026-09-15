<script>
  import {
    Message,
    MessageAction,
    MessageActions,
    MessageContent,
    MessageResponse,
  } from "$lib/components/ai-elements/message/index.js";

  import Copy from "@lucide/svelte/icons/copy";
  import Check from "@lucide/svelte/icons/check";
  import RefreshCcw from "@lucide/svelte/icons/refresh-ccw";
  import SparklesIcon from "@lucide/svelte/icons/sparkles";
  import { llmManager } from "@/runes/llm.svelte";
  import { memoryManager } from "@/runes/memory.svelte";
  import LoaderIcon from "@lucide/svelte/icons/loader-2";
  import FileText from "@lucide/svelte/icons/file-text";
  import { toast } from "$lib/toast.js";

  /**
   * @type {{ type?: "rag" | "chat" }}
   */
  let { type = "chat"} = $props();

  let messages = $derived(type === "rag" ? llmManager.ragMessages : llmManager.messages);
  let copiedIndex = $state(/** @type {number | null} */ (null));

  async function handleCopy(content, index) {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      copiedIndex = index;
      setTimeout(() => {
        if (copiedIndex === index) copiedIndex = null;
      }, 1500);
    } catch (e) {
      toast.error('Could not copy', e);
    }
  }

  function handleRetry(index) {
    if (type === "rag") return;
    llmManager.retryMessage(index);
  }

  let isBusy = $derived(type === "rag" ? llmManager.isRAGLoading : llmManager.isLoading);

  const starters = [
    'Summarise this writing in three sentences.',
    'What themes are emerging here?',
    'Suggest a stronger opening line.',
  ];

  function askStarter(prompt) {
    llmManager.sendMessage(prompt, 'Normal', memoryManager.context.content);
  }
</script>

<div class="flex flex-col gap-6 p-4">
  {#if messages.length === 0}
    <div class="flex flex-col items-center justify-center text-center gap-4 pt-24 pb-12 px-6">
      <div class="size-12 rounded-full bg-muted/40 flex items-center justify-center">
        <SparklesIcon strokeWidth={1.5} class="size-5 text-muted-foreground/60" />
      </div>
      <div class="space-y-1">
        <h3 class="text-lg font-normal">Ask about your writing</h3>
        <p class="text-sm text-muted-foreground/70 max-w-xs">
          Runs on your machine. Turn on the document toggle below to include the current writing as context.
        </p>
      </div>
      {#if type === "chat" && llmManager.modelsLoaded}
        <div class="flex flex-col gap-1.5 w-full max-w-xs mt-2">
          {#each starters as prompt (prompt)}
            <button
              type="button"
              onclick={() => askStarter(prompt)}
              disabled={isBusy}
              class="text-left text-sm px-3 py-2 rounded-lg border border-border/50 bg-muted/10 text-muted-foreground hover:text-foreground hover:bg-muted/30 transition-colors disabled:opacity-50"
            >
              {prompt}
            </button>
          {/each}
        </div>
      {:else if type === "chat" && !llmManager.modelsLoaded}
        <p class="text-xs text-muted-foreground/60">
          {llmManager.isLoadModelsInProgress
            ? `Loading model… ${llmManager.loadingProgress}%`
            : 'No AI model loaded yet. Use the sparkle button in the command bar to set one up.'}
        </p>
      {/if}
    </div>
  {/if}

  {#each messages as message, index (index)}
    <Message from={message.role} class="">

      <MessageContent class="">
        {#if message.role === "assistant"}
          {#if message.content === ""}
            <div class="flex items-center gap-2 text-muted-foreground">
              <LoaderIcon strokeWidth={1.5} class="size-4 animate-spin" />
              <span class="text-xs">Thinking...</span>
            </div>
          {:else}
            <MessageResponse content={message.content} class="" />
            {#if message.references && message.references.length > 0}
              <div class="mt-4 flex flex-wrap gap-2 border-t border-border/40 pt-4">
                <div class="w-full text-[0.625rem] uppercase font-bold text-muted-foreground/60 tracking-wider mb-1">
                  Sources
                </div>
                {#each message.references as ref (ref.title)}
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
          {#if type === "chat" && index === messages.length - 1}
            <MessageAction
              label="Retry"
              onclick={() => handleRetry(index)}
              tooltip="Regenerate response"
              class=""
            >
              <RefreshCcw class="size-3.5" />
            </MessageAction>
          {/if}

          <MessageAction
            label="Copy"
            onclick={() => handleCopy(message.content, index)}
            tooltip={copiedIndex === index ? "Copied" : "Copy to clipboard"}
            class=""
          >
            {#if copiedIndex === index}
              <Check class="size-3.5 text-success" />
            {:else}
              <Copy class="size-3.5" />
            {/if}
          </MessageAction>
        </MessageActions>
      {/if}
    </Message>
  {/each}

  {#if llmManager.error && !isBusy}
    <div class="p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-sm text-destructive">
      {llmManager.error}
    </div>
  {/if}
</div>
