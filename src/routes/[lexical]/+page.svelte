<script>
  export const prerender = false;
  import { fileManager } from "$lib/runes/fs.svelte";
  import MarkdownEditor from "$lib/components/md-editor.svelte";
  import { page } from "$app/state";

  const fileName = $derived(page.params.lexical);

  $effect(() => {
    if (!fileName) return;
    fileManager.readFile({
      name: fileName,
      path: fileName,
    });
  });

  /**
   * Handle save event from the editor
   * @param {string} content
   */
  function handleSave(content) {
      if (fileName){
          fileManager.saveCurrentFile(fileName, content)
      }
  }
</script>

<main>
  {#if fileManager.isLoading}
    <p>Loading document...</p>
  {:else}
      <div>
          <MarkdownEditor content={fileManager.currentContent} onSave={handleSave}/>
      </div>
  {/if}
</main>
