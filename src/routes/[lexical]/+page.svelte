<script>
  import MarkdownEditor from '$lib/components/md-editor.svelte';
  import EditorCommandbar from '@/components/editor-commandbar.svelte';
  import { appState } from "$lib/runes/app.svelte.js"

  /**
   * @type
   * {{ data: {
   *    fileName: string;
   *    content: string;
   * }}}
   */
  let { data } = $props();

  let { fileName, content } = $derived(data);

  let body = $derived(content || '');

  $effect(() => {
    body = content || '';
  });
</script>

<div class="grid grid-cols-[1fr_auto_1fr] w-full grid-rows-[1fr_auto] h-screen">
  <div class="row-span-1 col-span-1">
    {#if appState.ui.isHistoryOpen}
      <p>History open</p>
      {/if}
  </div>

  <div class="min-w-4xl max-w-4xl">
    <MarkdownEditor {fileName} {body} />
  </div>

  <div></div>

  <div class="sticky bottom-0 col-span-3 row-start-2">
    <EditorCommandbar {fileName} {body} />
  </div>
</div>
