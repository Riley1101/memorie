<script>
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { memoryManager } from '@/runes/memory.svelte.js';
  import AiChat from '$lib/components/ai-chat.svelte';
  import AiChatStream from '$lib/components/ai-chat-stream.svelte';
  import * as Resizable from '$lib/components/ui/resizable/index.js';

  /** @type {import("./$types").LayoutProps} */
  let { data, children } = $props();
  let { content, fileName } = $derived(data);

  console.log(data);

  $effect(() => {
    memoryManager.setContext(content, fileName);
  });
</script>

<Resizable.PaneGroup direction="horizontal" autoSaveId="main-layout" class="h-dvh overflow-hidden">
  <Resizable.Pane defaultSize={80}>
    <ScrollArea class="h-dvh w-full relative">
      {@render children()}
    </ScrollArea>
  </Resizable.Pane>
  <Resizable.Handle />
  <Resizable.Pane defaultSize={20} minSize={5} class="overflow-hidden grid grid-rows-[1fr_140px]">
    <ScrollArea class="h-[calc(100dvh-140px)] p-4 pb-0">
      <AiChatStream />
    </ScrollArea>
    <AiChat />
  </Resizable.Pane>
</Resizable.PaneGroup>
