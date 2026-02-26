<script>
  import { onMount } from 'svelte';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import { Separator } from '@/components/ui/separator/index.js';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import CpuIcon from '@lucide/svelte/icons/cpu';
  import SunIcon from '@lucide/svelte/icons/sun';
  import MoonIcon from '@lucide/svelte/icons/moon';
  import HomeIcon from '@lucide/svelte/icons/home';
  import { appState } from '$lib/runes/app.svelte.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { goto } from '$app/navigation';
  import { llmManager } from '$lib/runes/llm.svelte.js';

  onMount(() => {
    configManager.getConfig();
    llmManager.checkModels();
    llmManager.fetchSupportedModels();
  });
</script>

<div class="w-full h-full flex flex-col p-8 md:p-12 max-w-3xl mx-auto overflow-hidden">
  <div class="flex items-center justify-between mb-8">
    <h2 class="text-5xl font-normal text-foreground">Settings</h2>
    <Button 
      variant="ghost" 
      size="icon"
      class="opacity-20 hover:opacity-100 transition-opacity"
      onclick={() => goto('/')}
      disabled={false}
    >
      <HomeIcon class="size-5" />
    </Button>
  </div>

  <div class="flex-1 overflow-hidden w-full">
    <ScrollArea type="scroll" class="w-full h-full">
      <div class="space-y-12 pr-4">
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
                <span class="text-xs font-mono uppercase tracking-widest opacity-50">Supported models (Kalosm)</span>
              </div>

              {#if llmManager.error}
                <div class="mt-2 p-3 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive">
                  {llmManager.error}
                </div>
              {/if}

              <p class="text-sm text-muted-foreground mt-2">
                Default model: <strong>{llmManager.supportedModels.find(m => m.id === (configManager.config?.default_llm_model_id ?? 'qwen_2_5_1_5b_instruct'))?.name ?? (configManager.config?.default_llm_model_id ?? 'qwen_2_5_1_5b_instruct')}</strong>
              </p>
              
              <div class="space-y-3 mt-2">
                {#if llmManager.supportedModels.length > 0}
                  {#each llmManager.supportedModels as model}
                    <div class="flex items-center justify-between gap-3 p-3 rounded-md bg-background/50 border border-border/50">
                      <div class="flex flex-col min-w-0 flex-1">
                        <span class="text-sm font-medium truncate">{model.name}</span>
                        <div class="flex items-center gap-2 mt-1">
                          <span
                            class="inline-flex px-2 py-0.5 rounded text-[10px] font-medium uppercase tracking-wider {model.model_type === 'chat'
                              ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20'
                              : model.model_type === 'reasoning'
                                ? 'bg-violet-500/10 text-violet-600 dark:text-violet-400 border border-violet-500/20'
                                : 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20'}"
                          >
                            {model.model_type}
                          </span>
                        </div>
                      </div>
                      <div class="flex items-center gap-2 shrink-0 flex-wrap justify-end">
                        {#if model.downloaded}
                          <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-blue-500/10 border border-blue-500/20">
                            <div class="size-1.5 rounded-full bg-blue-500 animate-pulse"></div>
                            <span class="text-[10px] uppercase tracking-wider font-bold text-blue-500">Downloaded</span>
                          </div>
                          {@const isDefault = (configManager.config?.default_llm_model_id ?? 'qwen_2_5_1_5b_instruct') === model.id}
                          {#if !isDefault}
                            <Button
                              variant="ghost"
                              size="sm"
                              class="text-xs h-8"
                              onclick={() => configManager.setDefaultLlmModel(model.id)}
                            >
                              Set as default
                            </Button>
                          {:else}
                            <span class="text-[10px] uppercase tracking-wider font-medium text-muted-foreground">Default</span>
                          {/if}
                        {:else}
                          {@const isDownloading = llmManager.downloadingModelId === model.id}
                          <Button
                            variant="outline"
                            size="sm"
                            class="text-xs h-8"
                            onclick={() => llmManager.downloadModel(model.id)}
                            disabled={isDownloading || !!llmManager.downloadingModelId}
                          >
                            {#if isDownloading}
                              <span class="flex items-center gap-1.5">
                                <span class="size-3 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
                                Downloading {llmManager.loadingProgress}%
                              </span>
                            {:else}
                              Download
                            {/if}
                          </Button>
                        {/if}
                      </div>
                    </div>
                  {/each}
                {:else}
                  <div class="p-4 rounded-md bg-muted/10 border border-dashed border-border/50 flex flex-col items-center justify-center gap-2">
                    <div class="size-4 border-2 border-primary/20 border-t-primary rounded-full animate-spin"></div>
                    <span class="text-[10px] text-muted-foreground uppercase tracking-widest">Loading supported models…</span>
                  </div>
                {/if}
              </div>

              <p class="text-base text-muted-foreground mt-6 italic opacity-70">
                Models are stored locally in your app directory. Chat, reasoning, and coding models are supported. Only downloaded models can be set as default.
              </p>
            </div>
          </div>
        </section>
      </div>
    </ScrollArea>
  </div>
</div>
