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
  bind:open={() => appState.ui.isChatOpen, (newOpen) => {
    appState.toggleAiChat(newOpen);
  }}
  direction="right" class="absolute top-0 right-0 h-dvh"
>
  <Drawer.Content class="dark text-foreground md:min-w-[600px] font-serif">
    <ScrollArea class="w-full h-[calc(100dvh-140px)] p-4 pb-0">
      <AiChatStream />
    </ScrollArea>
    <Drawer.Footer>
      <AiChat />
    </Drawer.Footer>
  </Drawer.Content>
</Drawer.Root>
