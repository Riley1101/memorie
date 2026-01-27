<script>
  import { onMount } from 'svelte';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { Separator } from '@/components/ui/separator/index.js';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import CpuIcon from '@lucide/svelte/icons/cpu';
  import SunIcon from '@lucide/svelte/icons/sun';
  import MoonIcon from '@lucide/svelte/icons/moon';
  import { appState } from '$lib/runes/app.svelte.js';
  import { Button } from '$lib/components/ui/button/index.js';

  onMount(() => {
    configManager.getConfig();
  });
</script>

<div class="h-dvh flex flex-col">
  <div class="writing-surface flex-1 overflow-hidden flex flex-col">
    <h2 class="text-5xl font-normal mb-12">Settings</h2>
  
    <div class="flex-1 overflow-hidden">
      <ScrollArea type="scroll" class="h-full pr-4">
        <div class="space-y-12">
          <!-- Appearance Section -->
          <section>
            <div class="flex items-center gap-2 mb-6">
              <h3 class="text-2xl font-normal">Appearance</h3>
            </div>
            <div class="p-6 rounded-lg bg-muted/20 border border-border/50 flex items-center justify-between">
              <div>
                <p class="text-lg font-normal">Theme</p>
                <p class="text-sm text-muted-foreground mt-1 tracking-tight">
                  Switch between light and dark interface.
                </p>
              </div>
              <div class="flex bg-muted/40 p-1 rounded-md border border-border/50">
                <Button 
                  variant={appState.ui.theme === 'light' ? 'secondary' : 'ghost'} 
                  size="sm" 
                  class="gap-2 px-3 h-8 text-xs font-medium"
                  onclick={() => appState.setTheme('light')}
                  disabled={false}
                >
                  <SunIcon class="size-3.5" />
                  Light
                </Button>
                <Button 
                  variant={appState.ui.theme === 'dark' ? 'secondary' : 'ghost'} 
                  size="sm" 
                  class="gap-2 px-3 h-8 text-xs font-medium"
                  onclick={() => appState.setTheme('dark')}
                  disabled={false}
                >
                  <MoonIcon class="size-3.5" />
                  Dark
                </Button>
              </div>
            </div>
          </section>

          <Separator class="opacity-10" />

          <!-- App Paths Section -->
          <section>
            <div class="flex items-center gap-2 mb-8">
              <h3 class="text-2xl font-normal">Application Paths</h3>
            </div>
            
            <div class="grid gap-8">
              <div class="flex flex-col gap-1.5 p-6 rounded-lg bg-muted/20 border border-border/50">
                <div class="flex items-center gap-2 text-muted-foreground mb-1">
                  <FolderIcon class="size-4 opacity-50" />
                  <span class="text-xs font-mono uppercase tracking-widest opacity-50">Content Directory</span>
                </div>
                <code class="text-sm break-all font-mono text-primary">
                  {configManager.config?.content_directory || 'Loading...'}
                </code>
                <p class="text-base text-muted-foreground mt-3 italic opacity-70">
                  Where your markdown records are stored.
                </p>
              </div>

              <div class="flex flex-col gap-1.5 p-6 rounded-lg bg-muted/20 border border-border/50">
                <div class="flex items-center gap-2 text-muted-foreground mb-1">
                  <FolderIcon class="size-4 opacity-50" />
                  <span class="text-xs font-mono uppercase tracking-widest opacity-50">History Directory</span>
                </div>
                <code class="text-sm break-all font-mono text-primary">
                  {configManager.config?.undotree_dir || 'Loading...'}
                </code>
                <p class="text-base text-muted-foreground mt-3 italic opacity-70">
                  Where document undo/redo history is stored.
                </p>
              </div>
            </div>
          </section>

          <Separator class="opacity-10" />

          <!-- AI Section -->
          <section>
            <div class="flex items-center gap-2 mb-8">
              <h3 class="text-2xl font-normal">AI Configuration</h3>
            </div>
            
            <div class="grid gap-8">
              <div class="flex flex-col gap-1.5 p-6 rounded-lg bg-muted/20 border border-border/50">
                <div class="flex items-center gap-2 text-muted-foreground mb-1">
                  <CpuIcon class="size-4 opacity-50" />
                  <span class="text-xs font-mono uppercase tracking-widest opacity-50">Default LLM Model</span>
                </div>
                <code class="text-sm break-all font-mono text-primary">
                  {configManager.config?.default_llm_model || 'Loading...'}
                </code>
                <p class="text-base text-muted-foreground mt-3 italic opacity-70">
                  The local GGUF model path used for chat and intelligence.
                </p>
              </div>
            </div>
          </section>
        </div>
      </ScrollArea>
    </div>
  </div>
</div>
