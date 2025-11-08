<script>
  import * as InputGroup from '$lib/components/ui/input-group';
  import Button from '@/components/ui/button/button.svelte';
  import { fileManager } from '@/runes/fs.svelte';

  let fileName = 'task';
  let content = '';

  /**
   * Create  a new file
   */
  function createOrSave() {
    if (fileName) {
      fileManager.createNewFile(fileName, content).catch(() => {
        console.error('Error creating file');
      });
    }
  }

  let favourites = $derived(fileManager.files);
  console.log('Favourites:', favourites);
</script>

<div class="container mx-auto max-w-xl p-8 mt-12">
  <div class="flex flex-col justify-center gap-8 py-8">
    <h2 class="text-4xl">Start writing down your thoughts</h2>
    <div class="gap-1 flex flex-row items-center">
      <InputGroup.Root>
        <InputGroup.Input placeholder="task" bind:value={fileName} />
        <InputGroup.Addon align="inline-end">
          <InputGroup.Text>.md</InputGroup.Text>
        </InputGroup.Addon>
      </InputGroup.Root>
      <Button variant="outline" onclick={() => createOrSave()}>Create</Button>
    </div>
  </div>
</div>
