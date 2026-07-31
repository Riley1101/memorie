<script>
  import AiChat from '$lib/components/ai-chat.svelte';
  import AiChatStream from '$lib/components/ai-chat-stream.svelte';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte.js';
  import * as Drawer from '$lib/components/ui/drawer/index.js';

  /** @type {import("./$types").LayoutProps} */
  let { data, children } = $props();

  let { content, fileName } = $derived(data);

  $effect(() => {
    memoryManager.setContext(content, fileName);
  });
</script>

{@render children()}
<Drawer.Root
  dismissible={false}
  bind:open={
    () => appState.ui.isChatOpen,
    (newOpen) => {
      appState.toggleAiChat(newOpen);
    }
  }
  direction="right"
  class="absolute top-0 right-0 h-dvh"
>
  <Drawer.Content
    data-testid="ai-drawer-content"
    class="{appState.ui.theme} min-w-1/2 text-foreground font-writer h-full flex flex-col bg-background/95 backdrop-blur-sm"
  >
    <ScrollArea data-testid="ai-drawer-scroll" class="h-dvh overflow-hidden flex-1 grow shrink-0">
      <AiChatStream />
    </ScrollArea>
    <Drawer.Footer data-testid="ai-drawer-footer" class="">
      <AiChat />
    </Drawer.Footer>
  </Drawer.Content>
</Drawer.Root>
