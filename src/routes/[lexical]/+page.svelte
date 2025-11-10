<script>
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import MarkdownEditor from '$lib/components/md-editor.svelte';
  import EditorCommandbar from '$lib/components/editor-commandbar.svelte';
  import EditorHistory from '$lib/components/editor-history.svelte';
  import { appState } from '$lib/runes/app.svelte.js';

  let { data } = $props();

  let { fileName, content, history } = $derived(data);

  let body = $derived(content || '');

  $effect(() => {
    body = content || '';
  });
</script>

<div class="grid grid-cols-[auto_1fr_auto] md:grid-cols-[240px_auto_240px] w-full grid-rows-[1fr_auto] h-screen relative">
  {#if appState.ui.isHistoryOpen}
    <ScrollArea
      type="scroll"
      class="bg-card absolute lg:relative w-full max-h-[calc(100vh-2.5em)] left-0 top-0 border-r"
    >
      <EditorHistory {fileName} {history} />
    </ScrollArea>
  {/if}

  <ScrollArea type="scroll" class="col-start-2 w-full mx-auto max-h-[calc(100vh-2.5em)]">
    <MarkdownEditor {fileName} {body} />
  </ScrollArea>

  <ScrollArea type="scroll" class="max-h-[calc(100vh-2.5em)]">
    <div></div>
  </ScrollArea>

  <div class="sticky bottom-0 col-span-3 row-start-2">
    <EditorCommandbar {fileName} {body} currentVersion={history.current} />
  </div>
</div>
