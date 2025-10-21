<script>
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import { fileManager } from '$lib/runes/fs.svelte';
  import { invoke } from '@tauri-apps/api/core';

  let query = $state('');

  let { ref = $bindable(null), ...restProps } = $props();

  // Example functions to demonstrate invoking Tauri commands
  // eslint-disable-next-line no-unused-vars
  function handleSearchEmbeddings() {
    invoke('search_embeddings', { query }).catch((err) => {
      console.error('Error searching embeddings:', err);
    });
  }

  // eslint-disable-next-line no-unused-vars
  function handleCreateEmbeddings() {
    const currentContent = fileManager.currentContent;
    const currentFile = fileManager.currentFile;

    invoke('create_embeddings', { name: currentFile, content: currentContent })
      .then((res) => {
        console.log('Embeddings created:', res);
      })
      .catch((err) => {
        console.error('Error creating embeddings:', err);
      });
  }
</script>

<Sidebar.Root
  bind:ref
  side="right"
  collapsible="icon"
  class="sticky top-0 hidden h-svh border-r lg:flex"
  {...restProps}
>
  <Sidebar.Header></Sidebar.Header>
  <Sidebar.Content></Sidebar.Content>
  <Sidebar.Rail />
</Sidebar.Root>
