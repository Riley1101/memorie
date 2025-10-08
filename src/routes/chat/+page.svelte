<script>
  import { Button } from "$lib/components/ui/button";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Textarea } from "$lib/components/ui/textarea";
    import ModalSelect from "@/components/modal-select.svelte";

  function handleSend() {}
</script>

<main class="container w-full h-[90dvh] grid grid-rows-[1fr_auto] gap-4 py-8">
  <ScrollArea class="bg-card rounded-md border p-4">
    Lorem ipsum dolor sit amet consectetur adipisicing elit. Earum, repellat!
    Molestias at, culpa facilis et tempore numquam ipsa voluptatem nisi veniam
    quaerat suscipit unde reprehenderit, esse incidunt neque illo aliquid?
  </ScrollArea>
  <div class="bg-card flex flex-col gap-4 border p-4 rounded-md">
    <Textarea placeholder="Type your text here..." class="h-32" />
    <div class="flex gap-2 items-center self-end">
        <ModalSelect/>
        <Button onclick={handleSend}>Send</Button>
    </div>
  </div>
</main>
