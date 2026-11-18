<script>
  import AiChat from '$lib/components/ai-chat.svelte';
  import AiChatStream from '$lib/components/ai-chat-stream.svelte';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import { appState } from '$lib/runes/app.svelte.js';
  import PanelResizer from '$lib/components/panel-resizer.svelte';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import XIcon from '@lucide/svelte/icons/x';

  /** @type {HTMLElement | undefined} */
  let panel = $state();

  const MIN_WIDTH = 300;
  const MAX_WIDTH = 640;

  /** Dragged on the panel's inner edge and remembered across launches. */
  let width = $state(readWidth());

  function readWidth() {
    try {
      const saved = Number(localStorage.getItem('assistantPanelWidth'));
      if (Number.isFinite(saved) && saved > 0) {
        return Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, saved));
      }
    } catch {
      // Not remembered; the default stands.
    }
    return 372;
  }

  /** @param {number} next */
  function setWidth(next) {
    width = next;
    try {
      localStorage.setItem('assistantPanelWidth', String(next));
    } catch {
      // Not remembered; fine.
    }
  }

  /**
   * Escape closes the assistant, but only while the focus is inside it — the
   * key belongs to the editor (find, command mode) the rest of the time.
   * @param {KeyboardEvent} e
   */
  function handleEscape(e) {
    if (e.key !== 'Escape' || e.defaultPrevented) return;
    if (panel && e.target instanceof Node && panel.contains(e.target)) {
      appState.toggleAiChat(false);
    }
  }
</script>

<svelte:document onkeydown={handleEscape} />

<!-- The assistant is a panel in the row beside the writing, like the library
     and the scene panel — not a drawer over the page. -->
<aside
  bind:this={panel}
  data-testid="ai-drawer-content"
  aria-label="Writing assistant"
  class="well relative flex max-w-[calc(100vw-24px)] shrink-0 flex-col overflow-hidden rounded-group font-sans text-sm"
  style="width: {width}px"
>
  <PanelResizer
    {width}
    min={MIN_WIDTH}
    max={MAX_WIDTH}
    label="Resize the assistant"
    onresize={setWidth}
  />
  <header class="flex h-11 shrink-0 items-center gap-2 pl-4 pr-2 shadow-[0_1px_0_var(--hair)]">
    <SparklesIcon strokeWidth={1.75} class="size-3.5 text-primary" />
    <span class="flex-1 text-xs font-semibold text-muted-foreground">Assistant</span>
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
  <footer data-testid="ai-drawer-footer" class="shrink-0 p-3">
    <AiChat />
  </footer>
</aside>
