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
  import MonitorIcon from '@lucide/svelte/icons/monitor';
  import HomeIcon from '@lucide/svelte/icons/home';
  import AlignJustifyIcon from '@lucide/svelte/icons/align-justify';
  import Rows3Icon from '@lucide/svelte/icons/rows-3';
  import PaletteIcon from '@lucide/svelte/icons/palette';
  import CloudIcon from '@lucide/svelte/icons/cloud';
  import { appState, THEME_PALETTES } from '$lib/runes/app.svelte.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { llmManager } from '$lib/runes/llm.svelte.js';
  import { gitManager } from '$lib/runes/git.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import GithubIcon from '@lucide/svelte/icons/git-branch';
  import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
  import DownloadCloudIcon from '@lucide/svelte/icons/download-cloud';

  const SECTIONS = [
    { id: 'appearance', label: 'Appearance', icon: PaletteIcon },
    { id: 'ai', label: 'AI', icon: CpuIcon },
    { id: 'sync', label: 'Cloud Sync', icon: CloudIcon },
  ];

  let activeSection = $state('appearance');
  let repoInput = $state('');
  let commitMessage = $state('');

  onMount(() => {
    configManager.getConfig().then(() => {
      if (configManager.config?.ai_enabled) {
        llmManager.checkModels();
        llmManager.fetchSupportedModels();
      }
    });
    gitManager.checkSession().then(() => {
      if (gitManager.user) gitManager.refreshStatus();
    });
  });

  async function handleToggleAiEnabled() {
    const enabling = !(configManager.config?.ai_enabled ?? false);
    await configManager.setAiEnabled(enabling);
    if (enabling) {
      await llmManager.setupModels();
      await llmManager.fetchSupportedModels();
    }
  }

  $effect(() => {
    if (configManager.config?.github_repo && !repoInput) {
      repoInput = configManager.config.github_repo;
    }
  });

  async function handleSaveRepo() {
    if (!repoInput.trim()) return;
    await gitManager.setRepo(repoInput.trim());
    await gitManager.refreshStatus();
  }

  async function handleImport() {
    if (!repoInput.trim()) return;
    await gitManager.setRepo(repoInput.trim());
    await gitManager.pull();
    if (gitManager.importResult === 'success') {
      await fileManager.getRecents();
      await fileManager.getBinders();
    }
  }

  async function handleCommitPush() {
    if (!commitMessage.trim()) return;
    await gitManager.commitAndPush(commitMessage.trim());
    if (gitManager.pushResult === 'success') commitMessage = '';
  }
</script>

<div class="page-container w-full h-full flex flex-col overflow-hidden">
  <div class="flex items-center justify-between mb-8">
    <h2 class="text-5xl font-normal text-foreground">Settings</h2>
    <Button
      variant="ghost"
      size="icon"
      class="text-muted-foreground hover:text-foreground rounded-full transition-colors"
      onclick={() => goto(resolve('/'))}
      disabled={false}
    >
      <HomeIcon strokeWidth={1.5} class="size-5" />
    </Button>
  </div>

  <div class="flex-1 min-h-0 flex flex-col md:flex-row gap-6 md:gap-10">
    <!-- Side nav -->
    <nav class="flex md:flex-col gap-1 shrink-0 md:w-44 overflow-x-auto md:overflow-visible pb-2 md:pb-0">
      {#each SECTIONS as section (section.id)}
        <button
          type="button"
          onclick={() => (activeSection = section.id)}
          class="flex items-center gap-2 px-3 py-2 rounded-md text-sm text-left transition-colors shrink-0
            {activeSection === section.id
              ? 'bg-primary/10 text-primary font-medium'
              : 'text-muted-foreground hover:bg-muted/40 hover:text-foreground'}"
        >
          <section.icon strokeWidth={1.5} class="size-4 shrink-0" />
          <span>{section.label}</span>
        </button>
      {/each}
    </nav>

    <ScrollFade class="flex-1 min-w-0 overflow-hidden">
      <ScrollArea type="scroll" class="w-full h-full">
        <div class="space-y-12 pr-4 pb-4">
          {#if activeSection === 'appearance'}
            <!-- Appearance Section -->
            <section>
              <div class="flex items-center gap-2 mb-6">
                <h3 class="text-2xl font-normal">Appearance</h3>
              </div>
              <div class="grid gap-8">
                <div class="p-6 rounded-lg bg-muted/20 border border-border/50 flex flex-col gap-6 sm:flex-row sm:items-center sm:justify-between">
                  <div>
                    <p class="text-lg font-normal">Theme</p>
                    <p class="text-sm text-muted-foreground mt-1 tracking-tight">
                      Light, dark, or follow your system setting.
                    </p>
                  </div>
                  <div class="flex bg-muted/40 p-1 rounded-md border border-border/50 w-fit">
                    <Button
                      variant={appState.ui.themePreference === 'system' ? 'secondary' : 'ghost'}
                      size="sm"
                      class="gap-2 px-3 h-8 text-xs font-medium"
                      onclick={() => appState.setTheme('system')}
                      disabled={false}
                    >
                      <MonitorIcon strokeWidth={1.5} class="size-3.5" />
                      System
                    </Button>
                    <Button
                      variant={appState.ui.themePreference === 'light' ? 'secondary' : 'ghost'}
                      size="sm"
                      class="gap-2 px-3 h-8 text-xs font-medium"
                      onclick={() => appState.setTheme('light')}
                      disabled={false}
                    >
                      <SunIcon strokeWidth={1.5} class="size-3.5" />
                      Light
                    </Button>
                    <Button
                      variant={appState.ui.themePreference === 'dark' ? 'secondary' : 'ghost'}
                      size="sm"
                      class="gap-2 px-3 h-8 text-xs font-medium"
                      onclick={() => appState.setTheme('dark')}
                      disabled={false}
                    >
                      <MoonIcon strokeWidth={1.5} class="size-3.5" />
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
                      <Rows3Icon strokeWidth={1.5} class="size-3.5" />
                      Default
                    </Button>
                    <Button
                      variant={appState.ui.density === 'compact' ? 'secondary' : 'ghost'}
                      size="sm"
                      class="gap-2 px-3 h-8 text-xs font-medium"
                      onclick={() => appState.setDensity('compact')}
                      disabled={false}
                    >
                      <AlignJustifyIcon strokeWidth={1.5} class="size-3.5" />
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
              </div>
            </section>

            <Separator class="opacity-10" />

            <!-- App Paths Section -->
            <section>
              <div class="flex items-center gap-2 mb-8">
                <h3 class="text-2xl font-normal">Storage</h3>
              </div>

              <div class="grid gap-8">
                <div class="flex flex-col gap-1.5 p-6 rounded-lg bg-muted/20 border border-border/50">
                  <div class="flex items-center gap-2 text-muted-foreground mb-1">
                    <FolderIcon strokeWidth={1.5} class="size-4 opacity-50" />
                    <span class="text-xs font-mono uppercase tracking-widest opacity-50">Content Directory</span>
                  </div>
                  <code class="text-sm break-all font-mono text-foreground/90">
                    {configManager.config?.content_directory || 'Loading...'}
                  </code>
                  <p class="text-sm text-muted-foreground mt-3 tracking-tight">
                    Where your writings are stored as Markdown files.
                  </p>
                </div>

                <div class="flex flex-col gap-1.5 p-6 rounded-lg bg-muted/20 border border-border/50">
                  <div class="flex items-center gap-2 text-muted-foreground mb-1">
                    <FolderIcon strokeWidth={1.5} class="size-4 opacity-50" />
                    <span class="text-xs font-mono uppercase tracking-widest opacity-50">History Directory</span>
                  </div>
                  <code class="text-sm break-all font-mono text-foreground/90">
                    {configManager.config?.undotree_dir || 'Loading...'}
                  </code>
                  <p class="text-sm text-muted-foreground mt-3 tracking-tight">
                    Where version history for each writing is kept.
                  </p>
                </div>
              </div>
            </section>
          {:else if activeSection === 'ai'}
            <!-- AI Section -->
            <section>
              <div class="flex items-center gap-2 mb-8">
                <h3 class="text-2xl font-normal">AI Configuration</h3>
              </div>

              <div class="grid gap-8">
                <!-- Enable AI -->
                <div class="flex items-center justify-between gap-6 p-6 rounded-lg bg-muted/20 border border-border/50">
                  <div>
                    <p class="text-lg font-normal">Enable local AI</p>
                    <p class="text-sm text-muted-foreground mt-1 tracking-tight">
                      Turns on local AI chat, autocomplete, and model downloads. Off by default so
                      Memoire starts as a plain writing app.
                    </p>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={configManager.config?.ai_enabled ?? false}
                    onclick={handleToggleAiEnabled}
                    class="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors
                      {configManager.config?.ai_enabled ? 'bg-primary' : 'bg-muted-foreground/30'}"
                  >
                    <span
                      class="inline-block size-4 transform rounded-full bg-background transition-transform
                        {configManager.config?.ai_enabled ? 'translate-x-6' : 'translate-x-1'}"
                    ></span>
                  </button>
                </div>

                {#if configManager.config?.ai_enabled}
                <div class="flex flex-col gap-1.5 p-6 rounded-lg bg-muted/20 border border-border/50">
                  <div class="flex items-center gap-2 text-muted-foreground mb-1">
                    <CpuIcon strokeWidth={1.5} class="size-4 opacity-50" />
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
                                class="inline-flex px-2 py-0.5 rounded text-[0.625rem] font-medium uppercase tracking-wider {model.model_type === 'chat'
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
                                <span class="text-[0.625rem] uppercase tracking-wider font-bold text-info">Downloaded</span>
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
                                <span class="text-[0.625rem] uppercase tracking-wider font-medium text-muted-foreground">Default</span>
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
                                    <span class="size-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
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
                        <span class="text-[0.625rem] text-muted-foreground uppercase tracking-widest">Loading supported models…</span>
                      </div>
                    {/if}
                  </div>

                  <p class="text-sm text-muted-foreground mt-6 tracking-tight">
                    Models are stored locally in your app directory. Only downloaded models can be set as default.
                  </p>
                </div>

                <div class="flex flex-col gap-3 p-6 rounded-lg bg-muted/20 border border-border/50">
                  <div class="flex items-center gap-2 text-muted-foreground mb-1">
                    <CpuIcon strokeWidth={1.5} class="size-4 opacity-50" />
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
                {/if}
              </div>
            </section>
          {:else if activeSection === 'sync'}
            <!-- Cloud Sync Section -->
            <section>
              <div class="flex items-center gap-2 mb-8">
                <h3 class="text-2xl font-normal">Cloud Sync</h3>
              </div>

              <div class="grid gap-8">
                <!-- Account -->
                <div class="flex flex-col gap-3 p-6 rounded-lg bg-muted/20 border border-border/50">
                  <div class="flex items-center gap-2 text-muted-foreground mb-1">
                    <GithubIcon strokeWidth={1.5} class="size-4 opacity-50" />
                    <span class="text-xs font-mono uppercase tracking-widest opacity-50">Account</span>
                  </div>

                  {#if gitManager.error}
                    <div class="p-3 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive">
                      {typeof gitManager.error === 'string' ? gitManager.error : 'Something went wrong.'}
                    </div>
                  {/if}

                  {#if gitManager.isCheckingSession}
                    <div class="flex items-center gap-2 text-sm text-muted-foreground">
                      <span class="size-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
                      Checking session…
                    </div>
                  {:else if gitManager.user}
                    <div class="flex items-center justify-between gap-3">
                      <div class="flex items-center gap-3 min-w-0">
                        {#if gitManager.user.avatar_url}
                          <img src={gitManager.user.avatar_url} alt="" class="size-9 rounded-full" />
                        {/if}
                        <div class="flex flex-col min-w-0">
                          <span class="text-sm font-medium truncate">{gitManager.user.name || gitManager.user.login}</span>
                          <span class="text-xs text-muted-foreground truncate">@{gitManager.user.login}</span>
                        </div>
                      </div>
                      <Button variant="outline" size="sm" class="text-xs h-8" onclick={() => gitManager.logout()}>
                        Disconnect
                      </Button>
                    </div>
                  {:else if gitManager.deviceCode}
                    <div class="flex flex-col items-start gap-2">
                      <p class="text-sm text-muted-foreground">
                        Enter this code at
                        <span class="font-mono text-primary">{gitManager.deviceCode.verification_uri}</span>
                        (opened in your browser):
                      </p>
                      <span class="text-2xl font-mono tracking-widest font-medium">{gitManager.deviceCode.user_code}</span>
                      <div class="flex items-center gap-2 text-xs text-muted-foreground mt-1">
                        <span class="size-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
                        Waiting for approval…
                      </div>
                      <Button variant="ghost" size="sm" class="text-xs h-7 mt-1" onclick={() => gitManager.cancelLogin()}>
                        Cancel
                      </Button>
                    </div>
                  {:else}
                    <Button
                      variant="outline"
                      size="sm"
                      class="gap-2 text-xs h-8 w-fit"
                      onclick={() => gitManager.startLogin()}
                      disabled={gitManager.isConnecting}
                    >
                      <GithubIcon strokeWidth={1.5} class="size-3.5" />
                      Login with GitHub
                    </Button>
                  {/if}
                </div>

                {#if gitManager.user}
                  <!-- Auto push on exit -->
                  <div class="flex items-center justify-between gap-6 p-6 rounded-lg bg-muted/20 border border-border/50">
                    <div>
                      <p class="text-lg font-normal">Auto-push on exit</p>
                      <p class="text-sm text-muted-foreground mt-1 tracking-tight">
                        Commit & push any pending changes to GitHub when you close Memoire.
                      </p>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={configManager.config?.auto_push_on_exit ?? false}
                      onclick={() =>
                        configManager.setAutoPushOnExit(!(configManager.config?.auto_push_on_exit ?? false))}
                      class="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors
                        {configManager.config?.auto_push_on_exit ? 'bg-primary' : 'bg-muted-foreground/30'}"
                    >
                      <span
                        class="inline-block size-4 transform rounded-full bg-background transition-transform
                          {configManager.config?.auto_push_on_exit ? 'translate-x-6' : 'translate-x-1'}"
                      ></span>
                    </button>
                  </div>

                  <!-- Repository -->
                  <div class="flex flex-col gap-3 p-6 rounded-lg bg-muted/20 border border-border/50">
                    <div class="flex items-center gap-2 text-muted-foreground mb-1">
                      <GithubIcon strokeWidth={1.5} class="size-4 opacity-50" />
                      <span class="text-xs font-mono uppercase tracking-widest opacity-50">Repository</span>
                    </div>
                    <p class="text-sm text-muted-foreground">
                      Writing in your content directory is committed &amp; pushed to this GitHub repository.
                      Already have writing on GitHub? Import pulls it into this content directory.
                    </p>
                    <div class="flex items-center gap-2 mt-1">
                      <input
                        type="text"
                        bind:value={repoInput}
                        placeholder="owner/repo"
                        class="flex h-9 flex-1 rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
 />
                      <Button variant="secondary" size="sm" class="text-xs h-9" onclick={handleSaveRepo}>
                        Save
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        class="gap-1.5 text-xs h-9"
                        onclick={handleImport}
                        disabled={gitManager.isImporting || !repoInput.trim()}
                      >
                        {#if gitManager.isImporting}
                          <span class="size-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
                          Importing…
                        {:else}
                          <DownloadCloudIcon strokeWidth={1.5} class="size-3.5" />
                          Import
                        {/if}
                      </Button>
                    </div>
                    {#if gitManager.importResult === 'success'}
                      <div class="p-2 rounded-md bg-success/10 border border-success/20 text-sm text-success">
                        Imported from GitHub.
                      </div>
                    {:else if gitManager.importResult === 'error'}
                      <div class="p-2 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive">
                        Import failed: {typeof gitManager.error === 'string' ? gitManager.error : 'unknown error'}
                      </div>
                    {/if}
                  </div>

                  <!-- Status & push -->
                  <div class="flex flex-col gap-3 p-6 rounded-lg bg-muted/20 border border-border/50">
                    <div class="flex items-center justify-between mb-1">
                      <div class="flex items-center gap-2 text-muted-foreground">
                        <UploadCloudIcon strokeWidth={1.5} class="size-4 opacity-50" />
                        <span class="text-xs font-mono uppercase tracking-widest opacity-50">Changes</span>
                      </div>
                      <Button variant="ghost" size="sm" class="text-xs h-7" onclick={() => gitManager.refreshStatus()}>
                        Refresh
                      </Button>
                    </div>

                    {#if gitManager.isLoadingStatus}
                      <div class="flex items-center gap-2 text-sm text-muted-foreground">
                        <span class="size-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
                        Loading status…
                      </div>
                    {:else if gitManager.status.length === 0}
                      <p class="text-sm text-muted-foreground italic">No changes to commit.</p>
                    {:else}
                      <div class="flex flex-col gap-1 max-h-40 overflow-y-auto">
                        {#each gitManager.status as file (file.path)}
                          <div class="flex items-center justify-between gap-2 text-sm px-2 py-1 rounded bg-background/50">
                            <span class="font-mono truncate">{file.path}</span>
                            <span class="text-[0.625rem] uppercase tracking-wider text-muted-foreground shrink-0">{file.status}</span>
                          </div>
                        {/each}
                      </div>
                    {/if}

                    <textarea
                      class="flex min-h-20 w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring mt-2"
                      placeholder="Commit message"
                      bind:value={commitMessage}
                    ></textarea>

                    {#if gitManager.pushResult === 'success'}
                      <div class="p-2 rounded-md bg-success/10 border border-success/20 text-sm text-success">
                        Pushed to GitHub.
                      </div>
                    {/if}

                    <Button
                      class="gap-2 w-fit"
                      onclick={handleCommitPush}
                      disabled={gitManager.isPushing || !commitMessage.trim()}
                    >
                      {#if gitManager.isPushing}
                        <span class="size-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
                        Pushing…
                      {:else}
                        <UploadCloudIcon strokeWidth={1.5} class="size-3.5" />
                        Commit & Push
                      {/if}
                    </Button>
                  </div>
                {/if}
              </div>
            </section>
          {/if}
        </div>
      </ScrollArea>
    </ScrollFade>
  </div>
</div>
