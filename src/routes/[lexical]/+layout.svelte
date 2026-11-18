<script>
  import AiChat from '$lib/components/ai-chat.svelte';
  import AiChatStream from '$lib/components/ai-chat-stream.svelte';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import { appState } from '$lib/runes/app.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte.js';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import XIcon from '@lucide/svelte/icons/x';

  /** @type {import("./$types").LayoutProps} */
  let { data, children } = $props();

  let { content, fileName } = $derived(data);

  $effect(() => {
    memoryManager.setContext(content, fileName);
  });
</script>

{@render children()}
{#if configManager.config?.ai_enabled && appState.ui.isChatOpen}
  <!-- A floating panel beside the page rather than a modal drawer, so the writing stays in reach. -->
  <div
    role="dialog"
    aria-modal="false"
    tabindex="-1"
    data-testid="ai-drawer-content"
    aria-label="Writing assistant"
    class="glass fixed top-2 right-2 bottom-2 z-40 flex w-[min(372px,calc(100vw-16px))] flex-col overflow-hidden rounded-group font-sans text-sm text-popover-foreground"
    onkeydown={(e) => {
      if (e.key === 'Escape' && !e.defaultPrevented) appState.toggleAiChat(false);
    }}
  >
    <header class="flex h-11 shrink-0 items-center gap-2 pl-3.5 pr-1.5 shadow-[0_1px_0_var(--hair)]">
      <SparklesIcon strokeWidth={1.75} class="size-3.5 text-primary" />
      <span class="flex-1 font-semibold">Assistant</span>
      <button
        type="button"
        class="flex size-7 items-center justify-center rounded-full text-muted-foreground hover:bg-fill hover:text-foreground"
        aria-label="Close assistant"
        onclick={() => appState.toggleAiChat(false)}
      >
        <XIcon strokeWidth={1.75} class="size-3.5" />
      </button>
    </header>
    <ScrollFade class="min-h-0 flex-1 overflow-hidden">
      <ScrollArea data-testid="ai-drawer-scroll" class="h-full">
        <AiChatStream />
      </ScrollArea>
    </ScrollFade>
    <footer data-testid="ai-drawer-footer" class="shrink-0 p-2.5">
      <AiChat />
    </footer>
  </div>
{/if}
