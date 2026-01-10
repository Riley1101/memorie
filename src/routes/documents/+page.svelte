<script>
  import HomeFavourites from '$lib/components/home-favourites.svelte';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
</script>

<div class="container mx-auto max-w-3xl w-full h-dvh flex flex-col pt-8">
  <h2 class="text-4xl py-8">Documents</h2>
  <div class="overflow-hidden">
    <ScrollArea type="scroll" class="h-full">
      <HomeFavourites limit={50} />
    </ScrollArea>
  </div>
</div>
