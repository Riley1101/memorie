<script>
  import MinusIcon from "@lucide/svelte/icons/minus";
  import PlusIcon from "@lucide/svelte/icons/plus";
  import * as Drawer from "$lib/components/ui/drawer/index.js";
  import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
  import { appState } from "$lib/runes/app.svelte.js";


  let goal = $state(350);

  let open = $derived(appState.isAiChatOpen);

  function handleClick(adjustment) {
    goal = Math.max(200, Math.min(400, goal + adjustment));
  }

 
</script>
 
<Drawer.Root open={open}>
 <Drawer.Content>
  <div class="mx-auto w-full max-w-sm">
   <Drawer.Header>
    <Drawer.Title>Move Goal</Drawer.Title>
    <Drawer.Description>Set your daily activity goal.</Drawer.Description>
   </Drawer.Header>
   <div class="p-4 pb-0">
    <div class="flex items-center justify-center space-x-2">
     <Button
      variant="outline"
      size="icon"
      class="size-8 shrink-0 rounded-full"
      onclick={() => handleClick(-10)}
      disabled={goal <= 200}
     >
      <MinusIcon />
      <span class="sr-only">Decrease</span>
     </Button>
     <div class="flex-1 text-center">
      <div class="text-7xl font-bold tracking-tighter">
       {goal}
      </div>
      <div class="text-muted-foreground text-[0.70rem] uppercase">
       Calories/day
      </div>
     </div>
     <Button
      variant="outline"
      size="icon"
      class="size-8 shrink-0 rounded-full"
      onclick={() => handleClick(10)}
      disabled={goal >= 400}
     >
      <PlusIcon />
      <span class="sr-only">Increase</span>
     </Button>
    </div>
    <div class="mt-3 h-[120px]">
     <div class="h-full w-full">
       Lorem ipsum dolor sit amet, consectetur adipisicing elit. Iure eligendi tempore temporibus porro enim, distinctio numquam quas incidunt quasi? Earum!
     </div>
    </div>
   </div>
   <Drawer.Footer>
    <Button>Submit</Button>
    <Drawer.Close class={buttonVariants({ variant: "outline" })}
     >Cancel</Drawer.Close
    >
   </Drawer.Footer>
  </div>
 </Drawer.Content>
</Drawer.Root>
