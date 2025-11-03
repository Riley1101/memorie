<script>
  import * as InputGroup from '$lib/components/ui/input-group';
  import Button from '@/components/ui/button/button.svelte';
  import { fileManager } from '@/runes/fs.svelte';
  import { SidebarTrigger } from '@/components/ui/sidebar/index.js';

  let fileName = 'example';
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
</script>

<div class="container mx-auto max-w-4xl p-8">
  <div class="max-w-lg flex flex-col gap-8">
    <h2 class="text-4xl">Start writing down your thoughts</h2>
    <div class="flex flex-col">
      <InputGroup.Root>
        <InputGroup.Input placeholder="example.com" bind:value={fileName} />
        <InputGroup.Addon align="inline-end">
          <InputGroup.Text>.md</InputGroup.Text>
        </InputGroup.Addon>
      </InputGroup.Root>
      <Button class="mt-2 ml-auto" variant="outline" onclick={() => createOrSave()}>Create</Button>
    </div>
  </div>
</div>
