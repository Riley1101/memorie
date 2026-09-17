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
  import SearchIcon from "@lucide/svelte/icons/search";
  import MessageSquareIcon from "@lucide/svelte/icons/message-square";
  import FileText from "@lucide/svelte/icons/file-text";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { llmManager } from "@/runes/llm.svelte";
  import { memoryManager } from "@/runes/memory.svelte";
  import { configManager } from "@/runes/config.svelte.js";
  import { toast } from "$lib/toast.js";

  let messages = $derived(llmManager.messages);
  let isBusy = $derived(llmManager.isLoading);
  let copiedIndex = $state(/** @type {number | null} */ (null));

  let aiReady = $derived(
    configManager.config?.provider === "openrouter"
      ? configManager.hasOpenRouterApiKey
      : llmManager.modelsLoaded
  );

  let hasDocument = $derived(memoryManager.document !== null);

  let starters = $derived(
    hasDocument
      ? [
          "Summarize this document in three sentences",
          "Which parts of this feel slow or unclear?",
          "What recurring themes show up across my notes?",
        ]
      : [
          "What recurring themes show up across my notes?",
          "Help me brainstorm ideas for a new piece",
        ]
  );

  async function handleCopy(content, index) {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      copiedIndex = index;
      setTimeout(() => {
        if (copiedIndex === index) copiedIndex = null;
      }, 1500);
    } catch (e) {
      toast.error("Could not copy", e);
    }
  }

  function handleRetry(index) {
    llmManager.retryMessage(index, memoryManager.document);
  }

  function askStarter(prompt) {
    llmManager.sendMessage(prompt, memoryManager.document);
  }

  /** True while `message` is the in-flight assistant reply, so we can show a streaming cursor. */
  function isStreaming(message, index) {
    return isBusy && message.role === "assistant" && index === messages.length - 1;
  }

  /** `Novel/Chapter 3.md` -> `Chapter 3` */
  function noteLabel(title) {
    return (title ?? "").replace(/\.md$/, "").split("/").pop() || "Untitled";
  }

  function openNote(title) {
    goto(resolve(`/${encodeURIComponent(title)}`));
  }

  /** What to show before the first token arrives, based on how the reply was routed. */
  function pendingLabel(message) {
    switch (message.route?.intent) {
      case "search_notes":
        return message.references ? "Writing an answer…" : "Searching your notes…";
      case "current_document":
        return "Reading this document…";
      default:
        return null;
    }
  }
</script>

<div class="flex flex-col gap-6 p-4">
  {#if messages.length === 0}
    <div class="flex flex-col items-center justify-center text-center gap-5 pt-20 pb-12 px-6">
      <div class="size-12 rounded-full bg-primary/10 flex items-center justify-center">
        <SparklesIcon strokeWidth={1.5} class="size-5 text-primary/70" />
      </div>

      <div class="space-y-1">
        <h3 class="font-writer text-lg font-normal">Your writing assistant</h3>
        <p class="text-sm text-muted-foreground/70 max-w-xs">
          {configManager.config?.provider === "openrouter"
            ? "Answers via OpenRouter. Note search stays on this device."
            : "Runs entirely on this device."}
        </p>
      </div>

      <ul class="flex flex-col gap-2 w-full max-w-xs text-left text-sm text-muted-foreground">
        <li class="flex items-center gap-2.5">
          <FileText strokeWidth={1.5} class="size-4 shrink-0 text-primary/60" />
          <span>Ask about the document you have open</span>
        </li>
        <li class="flex items-center gap-2.5">
          <SearchIcon strokeWidth={1.5} class="size-4 shrink-0 text-primary/60" />
          <span>Find anything across your notes</span>
        </li>
        <li class="flex items-center gap-2.5">
          <MessageSquareIcon strokeWidth={1.5} class="size-4 shrink-0 text-primary/60" />
          <span>Or just talk through an idea</span>
        </li>
      </ul>

      {#if memoryManager.indexProgress}
        <p class="text-xs text-muted-foreground/60">
          {memoryManager.indexProgress.total
            ? `Indexing notes… ${memoryManager.indexProgress.done} of ${memoryManager.indexProgress.total}`
            : "Indexing notes…"}
        </p>
      {:else if memoryManager.indexStatus}
        <p class="text-xs text-muted-foreground/60">
          {memoryManager.indexStatus.documents === 1
            ? "1 note searchable"
            : `${memoryManager.indexStatus.documents} notes searchable`}
        </p>
      {/if}

      {#if aiReady}
        <div class="flex flex-col gap-1.5 w-full max-w-xs">
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
        <p class="text-xs text-muted-foreground/50">
          It works out what you mean. Start with <code>/search</code> or <code>/doc</code> to choose.
        </p>
      {:else}
        <p class="text-xs text-muted-foreground/60">
          {#if configManager.config?.provider === "openrouter"}
            No OpenRouter API key set. Add one in Settings → AI.
          {:else if llmManager.isLoadModelsInProgress}
            Loading model… {llmManager.loadingProgress}%
          {:else}
            No AI model loaded yet. Use the sparkle button in the command bar to set one up.
          {/if}
        </p>
      {/if}
    </div>
  {/if}

  {#each messages as message, index (index)}
    <Message from={message.role} class="">
      <div class="flex w-full items-start gap-2.5">
        {#if message.role === "assistant"}
          <div class="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
            <SparklesIcon strokeWidth={1.5} class="size-3.5" />
          </div>
        {/if}

        <div class="flex min-w-0 flex-1 flex-col gap-2 {message.role === 'user' ? 'items-end' : ''}">
          {#if message.role === "assistant" && message.route?.intent === "search_notes"}
            <div class="flex items-center gap-1.5 text-xs text-muted-foreground/70 font-sans">
              <SearchIcon strokeWidth={1.5} class="size-3" />
              <span class="truncate">
                Searched notes for “{message.route.query}”{message.references?.length === 0 ? " · no matches" : ""}
              </span>
            </div>
          {:else if message.role === "assistant" && message.route?.intent === "current_document"}
            <div class="flex items-center gap-1.5 text-xs text-muted-foreground/70 font-sans">
              <FileText strokeWidth={1.5} class="size-3" />
              <span class="truncate">Read “{noteLabel(message.route.documentTitle)}”</span>
            </div>
          {/if}

          <MessageContent class="">
            {#if message.role === "assistant"}
              {#if message.content === ""}
                {@const label = pendingLabel(message)}
                {#if label}
                  <div class="flex items-center gap-1.5 text-muted-foreground">
                    <span class="size-1.5 rounded-full bg-primary/60 animate-pulse"></span>
                    <span class="text-sm">{label}</span>
                  </div>
                {:else}
                  <div class="flex items-center gap-1.5 h-5 py-1">
                    <span class="size-1.5 rounded-full bg-muted-foreground/50 animate-bounce [animation-delay:0ms]"></span>
                    <span class="size-1.5 rounded-full bg-muted-foreground/50 animate-bounce [animation-delay:150ms]"></span>
                    <span class="size-1.5 rounded-full bg-muted-foreground/50 animate-bounce [animation-delay:300ms]"></span>
                  </div>
                {/if}
              {:else}
                <MessageResponse
                  content={message.content + (isStreaming(message, index) ? " ▍" : "")}
                  class=""
                />
              {/if}

              {#if message.references && message.references.length > 0}
                <div class="mt-3 flex flex-wrap items-center gap-1.5 border-t border-border/40 pt-3 font-sans">
                  <span class="text-[0.625rem] uppercase font-semibold text-muted-foreground/60 tracking-wider mr-1">
                    {message.references.length > 1 ? "Sources" : "Source"}
                  </span>
                  {#each message.references as ref (ref.title)}
                    {@const isOpen = ref.title === memoryManager.context.fileName}
                    <button
                      type="button"
                      title={ref.title}
                      onclick={() => openNote(ref.title)}
                      disabled={isOpen}
                      class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-secondary/30 text-xs text-muted-foreground border border-border/40 hover:bg-secondary/60 hover:text-foreground transition-colors disabled:cursor-default disabled:hover:bg-secondary/30 disabled:hover:text-muted-foreground"
                    >
                      <FileText strokeWidth={1.5} class="size-3.5 opacity-60" />
                      <span>{noteLabel(ref.title)}</span>
                      {#if isOpen}
                        <span class="opacity-60">· open</span>
                      {/if}
                    </button>
                  {/each}
                </div>
              {/if}
            {:else}
              <div class="whitespace-pre-wrap">{message.content}</div>
            {/if}
          </MessageContent>

          {#if message.role === "assistant" && message.content !== "" && !isStreaming(message, index)}
            <MessageActions class="opacity-0 transition-opacity duration-150 group-hover:opacity-100 focus-within:opacity-100">
              {#if index === messages.length - 1}
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
        </div>
      </div>
    </Message>
  {/each}

  {#if llmManager.error && !isBusy}
    <div class="p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-sm text-destructive">
      {llmManager.error}
    </div>
  {/if}
</div>
