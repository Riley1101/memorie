<script>
  import { Button } from '$lib/components/ui/button';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import { Textarea } from '$lib/components/ui/textarea';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let load_status = $state('');
  let response = $state('');
  let prompt = $state('Hi, How are you?');

  function handleLoadModel() {
    invoke('load_model')
      .then((res) => {
        load_status = res;
        console.log(load_status);
      })
      .catch((error) => {
        console.error(error);
      });
  }

  function handleSend() {
    invoke('run_chat', {
      message: prompt,
    })
      .then((res) => {
        response = res;
      })
      .catch((error) => {
        console.error(error);
      });
  }

  listen('chat-in-progress', (event) => {
    if (event?.payload?.content) {
      response += event.payload.content;
    }
  });
</script>

<main class="container w-full h-[90dvh] grid grid-rows-[1fr_auto] gap-4 py-8">
  <ScrollArea class="bg-card rounded-md border p-4">
    <h1>
      {load_status}
      {response}
    </h1>
  </ScrollArea>
  <div class="bg-card flex flex-col gap-4 border p-4 rounded-md">
    <Textarea placeholder="Type your text here..." class="h-32" bind:value={prompt} />
    <div class="flex gap-2 items-center self-end">
      <Button onclick={handleLoadModel}>Load Model</Button>
      <Button onclick={handleSend}>Send</Button>
    </div>
  </div>
</main>
