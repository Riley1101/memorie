<script>
  import { fileManager } from '@/runes/fs.svelte';
  import FileIcon from '@lucide/svelte/icons/file';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { Input } from '@/components/ui/input/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import Button from './ui/button/button.svelte';
  import ScrollFade from './scroll-fade.svelte';

  let { activeBinder = $bindable(null) } = $props();

  let isNewBinderDialogOpen = $state(false);
  let newBinderName = $state('');

  function selectBinder(name) {
    activeBinder = activeBinder === name ? null : name;
  }

  async function handleCreateBinder() {
    if (!newBinderName.trim()) return;
    const name = newBinderName.trim();
    await fileManager.createBinder(name);
    activeBinder = name;
    newBinderName = '';
    isNewBinderDialogOpen = false;
  }
</script>

<!-- Desktop: compact, borderless list. The top spacer mirrors the page header's
     height/margin exactly so the "Binders" label lands level with the search box. -->
<aside class="hidden md:flex w-44 shrink-0 h-fit max-h-[calc(100%-2rem)] mb-4 ml-4 flex-col font-writer">
  <div class="pt-8 invisible" aria-hidden="true">
    <h2 class="text-3xl md:text-5xl font-normal">&nbsp;</h2>
  </div>
  <div class="mb-4 md:mb-8"></div>
  <div class="px-2 pb-1.5 shrink-0">
    <h2 class="text-[9px] font-bold uppercase tracking-widest text-muted-foreground/50">Binders</h2>
  </div>
  <ScrollFade class="min-h-0" fadeSize="h-4">
    <ScrollArea type="scroll" class="min-h-0">
      <nav class="flex flex-col gap-0.5 px-2 pb-1.5">
        <button
            onclick={() => activeBinder = null}
            class="flex items-center gap-1.5 px-2 py-1 rounded-md text-[13px] text-left transition-colors {activeBinder === null ? 'bg-primary/10 text-primary font-medium' : 'text-muted-foreground/80 hover:bg-muted/30 hover:text-foreground'}"
        >
            <FileIcon class="size-3 shrink-0" />
            <span class="truncate">All Writings</span>
        </button>
        {#each fileManager.binders as binder (binder)}
            <button
                onclick={() => selectBinder(binder)}
                class="flex items-center gap-1.5 px-2 py-1 rounded-md text-[13px] text-left transition-colors {activeBinder === binder ? 'bg-primary/10 text-primary font-medium' : 'text-muted-foreground/80 hover:bg-muted/30 hover:text-foreground'}"
            >
                <FolderIcon class="size-3 shrink-0" />
                <span class="truncate">{binder}</span>
            </button>
        {/each}
      </nav>
    </ScrollArea>
  </ScrollFade>
  <div class="p-2 shrink-0">
    <button
        onclick={() => isNewBinderDialogOpen = true}
        class="w-full flex items-center gap-1.5 px-2 py-1 rounded-md text-[13px] text-muted-foreground/60 hover:bg-muted/30 hover:text-foreground transition-colors"
    >
        <FolderPlusIcon class="size-3 shrink-0" />
        <span>New Binder</span>
    </button>
  </div>
</aside>

<!-- Mobile: horizontal scrollable chip strip in place of the rail -->
<div class="md:hidden flex items-center gap-2 overflow-x-auto px-4 pt-3 pb-2 border-b border-border/20 font-writer">
  <button
      onclick={() => activeBinder = null}
      class="shrink-0 flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium border transition-colors {activeBinder === null ? 'bg-primary/10 border-primary/40 text-primary' : 'border-border/40 text-muted-foreground/70 hover:bg-muted/20'}"
  >
      All
  </button>
  {#each fileManager.binders as binder (binder)}
      <button
          onclick={() => selectBinder(binder)}
          class="shrink-0 flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium border transition-colors {activeBinder === binder ? 'bg-primary/10 border-primary/40 text-primary' : 'border-border/40 text-muted-foreground/70 hover:bg-muted/20'}"
      >
          <FolderIcon class="size-3" />
          <span class="truncate max-w-32">{binder}</span>
      </button>
  {/each}
  <button
      onclick={() => isNewBinderDialogOpen = true}
      class="shrink-0 flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium border border-dashed border-border/40 text-muted-foreground/60 hover:bg-muted/20 hover:text-foreground transition-colors"
  >
      <FolderPlusIcon class="size-3" />
      <span>New Binder</span>
  </button>
</div>

<Dialog.Root bind:open={isNewBinderDialogOpen}>
  <Dialog.Content class="sm:max-w-[400px] font-writer" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal">New Binder</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
        Binders group your writings into folders.
      </Dialog.Description>
    </Dialog.Header>
    <Input
        bind:value={newBinderName}
        type="text"
        placeholder="Binder name"
        class="mt-2"
        onkeydown={(e) => { if (e.key === 'Enter') handleCreateBinder(); }}
    />
    <Dialog.Footer class="mt-6 flex gap-2">
      <Button variant="ghost" onclick={() => { isNewBinderDialogOpen = false; newBinderName = ''; }} class="flex-1" disabled={false}>Cancel</Button>
      <Button variant="default" onclick={handleCreateBinder} class="flex-1" disabled={false}>Create</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
