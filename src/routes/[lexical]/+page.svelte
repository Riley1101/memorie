<script>
  import LexicalEditor from "$lib/components/lexical-editor.svelte";
  import { fileManager } from "$lib/runes/fs.svelte";
  import { page } from "$app/state";

  const fileName = $derived(page.params.lexical);

  $effect(() => {
    if (!fileName) return;
    fileManager.readFile({
      name: fileName,
      path: fileName,
    });
  });
</script>

<main>
  {#if fileManager.isLoading}
    <p>Loading document...</p>
  {:else if fileManager.currentContent}
    <LexicalEditor nodes={fileManager.currentContent} />
  {:else}
    <p>No content to display. Select a file to get started.</p>
  {/if}
</main>
