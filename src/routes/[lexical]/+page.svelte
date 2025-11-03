
<script>
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import MarkdownEditor from '$lib/components/md-editor.svelte';
  import EditorCommandbar from '$lib/components/editor-commandbar.svelte';
  import EditorHistory from '$lib/components/editor-history.svelte';
  import { appState } from "$lib/runes/app.svelte.js"


  let { data } = $props();

  let { fileName, content , history } = $derived(data);

  let body = $derived(content || '');

  $effect(() => {
    body = content || '';
  });

</script>

<div class="grid grid-cols-[1fr_auto_1fr] w-full grid-rows-[1fr_auto] h-screen relative">

  <ScrollArea class="w-full max-h-[calc(100vh-2.5em)]">
     {#if appState.ui.isHistoryOpen}
       <EditorHistory {fileName} {history}/>
     {/if}
  </ScrollArea>

  <ScrollArea class="w-full max-h-[calc(100vh-2.5em)]">
    <div class="min-w-4xl max-w-4xl">
      <MarkdownEditor {fileName} {body} />
    </div>
  </ScrollArea>

  <ScrollArea class="w-full max-h-[calc(100vh-2.5em)]">
    <div></div>
  </ScrollArea>

  <div class="sticky bottom-0 col-span-3 row-start-2">
    <EditorCommandbar {fileName} {body} />
  </div>
</div>
