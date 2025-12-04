<script>
  import { Separator } from '$lib/components/ui/separator/index.js';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import * as InputGroup from '$lib/components/ui/input-group';
  import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
  import HomeFavourites from '$lib/components/home-favourites.svelte';
  import { fileManager } from '@/runes/fs.svelte';
  import { invoke } from '@tauri-apps/api/core';

  let commandInput = $state("")
  let result = $state(null);

  /**
   * Create  a new file
   */
  function createOrSave() {
    if (commandInput) {
      fileManager.createNewFile(commandInput, "").catch(() => {
        console.error('Error creating file');
      });
    }
  }

  async function handleSubmit(){
    result ="Submitting .."
    invoke("search_documents",{
      query: commandInput
    }).then(res=>{
      result = res;
    }).catch(e=>{
      result = `Error: ${e}`
    })
  }

</script>

<div class="p-4 overflow-hidden container mx-auto max-w-3xl w-full h-dvh grid grid-rows-[1fr_auto]">
  <div class="pt-24 overflow-hidden">
    {#if commandInput===""}
    <ScrollArea type="scroll" class="h-full">
      <h2 class="text-4xl mb-8">Start writing down your thoughts</h2>
      <HomeFavourites />
    </ScrollArea>
    {:else}
      <div>
        {JSON.stringify(result)}
      </div>
    {/if}
  </div>

  <div class="flex flex-col gap-2">
    <InputGroup.Root>
      <InputGroup.Textarea placeholder="Ask, Search or Chat..." class="!text-base" bind:value={commandInput}/>
      <InputGroup.Addon align="block-end">
        <Separator orientation="vertical" class="!h-4" />
        <InputGroup.Button
          onclick={()=>{
             handleSubmit()
            }
          }
          variant="default"
          class="ml-auto rounded-full"
          size="icon-xs"
        >
          <ArrowUpIcon />
          <span class="sr-only">Send</span>
        </InputGroup.Button>
      </InputGroup.Addon>
    </InputGroup.Root>
  </div>
</div>
