<script>
  import { onMount } from 'svelte';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import { Separator } from '@/components/ui/separator/index.js';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import CpuIcon from '@lucide/svelte/icons/cpu';
  import SunIcon from '@lucide/svelte/icons/sun';
  import MoonIcon from '@lucide/svelte/icons/moon';
  import HomeIcon from '@lucide/svelte/icons/home';
  import AlignJustifyIcon from '@lucide/svelte/icons/align-justify';
  import Rows3Icon from '@lucide/svelte/icons/rows-3';
  import { appState, THEME_PALETTES } from '$lib/runes/app.svelte.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { llmManager } from '$lib/runes/llm.svelte.js';

  onMount(() => {
    configManager.getConfig();
    llmManager.checkModels();
    llmManager.fetchSupportedModels();
  });
</script>

<div class="page-container w-full h-full flex flex-col overflow-hidden">
  <div class="flex items-center justify-between mb-8">
    <h2 class="text-5xl font-normal text-foreground">Settings</h2>
    <Button 
      variant="ghost" 
      size="icon"
      class="text-muted-foreground hover:text-foreground transition-colors"
      onclick={() => goto(resolve('/'))}
      disabled={false}
    >
      <HomeIcon class="size-5" />
    </Button>
  </div>

  <ScrollFade class="flex-1 overflow-hidden w-full">
    <ScrollArea type="scroll" class="w-full h-full">
      <div class="space-y-12 pr-4">
        <!-- Appearance Section -->
        <section>
          <div class="flex items-center gap-2 mb-6">
            <h3 class="text-2xl font-normal">Appearance</h3>
          </div>
          <div class="p-6 rounded-lg bg-muted/20 border border-border/50 flex flex-col gap-6 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <p class="text-lg font-normal">Theme</p>
              <p class="text-sm text-muted-foreground mt-1 tracking-tight">
                Switch between light and dark interface.
              </p>
            </div>
            <div class="flex bg-muted/40 p-1 rounded-md border border-border/50 w-fit">
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

          <div class="p-6 rounded-lg bg-muted/20 border border-border/50 flex flex-col gap-6 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <p class="text-lg font-normal">Density</p>
              <p class="text-sm text-muted-foreground mt-1 tracking-tight">
                Overall text size and spacing across the app.
              </p>
            </div>
            <div class="flex bg-muted/40 p-1 rounded-md border border-border/50 w-fit">
              <Button
                variant={appState.ui.density === 'default' ? 'secondary' : 'ghost'}
                size="sm"
                class="gap-2 px-3 h-8 text-xs font-medium"
                onclick={() => appState.setDensity('default')}
                disabled={false}
              >
                <Rows3Icon class="size-3.5" />
                Default
              </Button>
              <Button
                variant={appState.ui.density === 'compact' ? 'secondary' : 'ghost'}
                size="sm"
                class="gap-2 px-3 h-8 text-xs font-medium"
                onclick={() => appState.setDensity('compact')}
                disabled={false}
              >
                <AlignJustifyIcon class="size-3.5" />
                Compact
              </Button>
            </div>
          </div>

          <div class="p-6 rounded-lg bg-muted/20 border border-border/50">
            <p class="text-lg font-normal">Color palette</p>
            <p class="text-sm text-muted-foreground mt-1 mb-4 tracking-tight">
              Accent and primary colors (shadcn-style).
            </p>
            <div class="flex flex-wrap gap-2">
              {#each THEME_PALETTES as palette (palette)}
                <button
                  type="button"
                  onclick={() => appState.setThemePalette(palette)}
                  class="flex items-center gap-2 px-3 py-2 rounded-md border text-sm font-medium transition-colors
                    {appState.ui.themePalette === palette
                      ? 'bg-primary text-primary-foreground border-primary'
                      : 'bg-background hover:bg-accent border-border'}"
                >
                  <span
                    class="size-3.5 rounded-full shrink-0
                      {palette === 'default' ? 'bg-neutral-500' : ''}
                      {palette === 'zinc' ? 'bg-zinc-500' : ''}
                      {palette === 'slate' ? 'bg-slate-500' : ''}
                      {palette === 'rose' ? 'bg-rose-500' : ''}
                      {palette === 'blue' ? 'bg-blue-500' : ''}
                      {palette === 'green' ? 'bg-green-500' : ''}
                      {palette === 'violet' ? 'bg-violet-500' : ''}"
                    aria-hidden="true"
                  ></span>
                  <span class="capitalize">{palette}</span>
                </button>
              {/each}
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
                Where writing undo/redo history is stored.
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
                  {#each llmManager.supportedModels as model (model.id)}
                    <div class="flex items-center justify-between gap-3 p-3 rounded-md bg-background/50 border border-border/50">
                      <div class="flex flex-col min-w-0 flex-1">
                        <span class="text-sm font-medium truncate">{model.name}</span>
                        <div class="flex items-center gap-2 mt-1">
                          <span
                            class="inline-flex px-2 py-0.5 rounded text-[10px] font-medium uppercase tracking-wider {model.model_type === 'chat'
                              ? 'bg-success/10 text-success border border-success/20'
                              : model.model_type === 'reasoning'
                                ? 'bg-primary/10 text-primary border border-primary/20'
                                : 'bg-warning/10 text-warning border border-warning/20'}"
                          >
                            {model.model_type}
                          </span>
                        </div>
                      </div>
                      <div class="flex items-center gap-2 shrink-0 flex-wrap justify-end">
                        {#if model.downloaded}
                          <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-info/10 border border-info/20">
                            <div class="size-1.5 rounded-full bg-info animate-pulse"></div>
                            <span class="text-[10px] uppercase tracking-wider font-bold text-info">Downloaded</span>
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

            <div class="flex flex-col gap-3 p-6 rounded-lg bg-muted/20 border border-border/50">
              <div class="flex items-center gap-2 text-muted-foreground mb-1">
                <CpuIcon class="size-4 opacity-50" />
                <span class="text-xs font-mono uppercase tracking-widest opacity-50">System Prompt</span>
              </div>
              <p class="text-sm text-muted-foreground">
                Set the instructions that guide the behavior of the AI in the sidebar chat.
              </p>
              
              <div class="flex flex-col gap-2 mt-2">
                <textarea
                  class="flex min-h-[120px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                  placeholder="e.g. You are a helpful AI assistant..."
                  value={configManager.config?.system_prompt ?? ''}
                  onchange={(e) => {
                    const val = e.currentTarget.value;
                    configManager.setSystemPrompt(val);
                  }}
                ></textarea>
              </div>
            </div>
          </div>
        </section>
      </div>
    </ScrollArea>
  </ScrollFade>
</div>
