<script>
  import { onMount } from 'svelte';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { Separator } from '@/components/ui/separator/index.js';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import CpuIcon from '@lucide/svelte/icons/cpu';

  onMount(() => {
    configManager.getConfig();
  });
</script>

<div class="p-4 container mx-auto max-w-3xl w-full h-dvh flex flex-col">
  <h2 class="text-4xl py-8">Settings</h2>
  
  <div class="overflow-hidden flex-1">
    <ScrollArea type="scroll" class="h-full pr-4">
      <div class="space-y-8">
        <!-- App Paths Section -->
        <section>
          <div class="flex items-center gap-2 mb-4">
            <h3 class="text-xl font-medium">Application Paths</h3>
          </div>
          
          <div class="grid gap-6">
            <div class="flex flex-col gap-1.5 p-4 rounded-lg bg-muted/40 border border-border/50">
              <div class="flex items-center gap-2 text-muted-foreground mb-1">
                <FolderIcon class="size-4" />
                <span class="text-xs font-mono uppercase tracking-wider">Content Directory</span>
              </div>
              <code class="text-sm break-all font-mono text-primary">
                {configManager.config?.content_directory || 'Loading...'}
              </code>
              <p class="text-base text-muted-foreground mt-2 italic">
                Where your markdown records are stored.
              </p>
            </div>

            <div class="flex flex-col gap-1.5 p-4 rounded-lg bg-muted/40 border border-border/50">
              <div class="flex items-center gap-2 text-muted-foreground mb-1">
                <FolderIcon class="size-4" />
                <span class="text-xs font-mono uppercase tracking-wider">History Directory</span>
              </div>
              <code class="text-sm break-all font-mono text-primary">
                {configManager.config?.undotree_dir || 'Loading...'}
              </code>
              <p class="text-base text-muted-foreground mt-2 italic">
                Where document undo/redo history is stored.
              </p>
            </div>
          </div>
        </section>

        <Separator class="opacity-30" />

        <!-- AI Section -->
        <section>
          <div class="flex items-center gap-2 mb-4">
            <h3 class="text-xl font-medium">AI Configuration</h3>
          </div>
          
          <div class="grid gap-6">
            <div class="flex flex-col gap-1.5 p-4 rounded-lg bg-muted/40 border border-border/50">
              <div class="flex items-center gap-2 text-muted-foreground mb-1">
                <CpuIcon class="size-4" />
                <span class="text-xs font-mono uppercase tracking-wider">Default LLM Model</span>
              </div>
              <code class="text-sm break-all font-mono text-primary">
                {configManager.config?.default_llm_model || 'Loading...'}
              </code>
              <p class="text-base text-muted-foreground mt-2 italic">
                The local GGUF model path used for chat and intelligence.
              </p>
            </div>
          </div>
        </section>
      </div>
    </ScrollArea>
  </div>
</div>
