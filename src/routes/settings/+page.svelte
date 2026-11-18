<script>
  import { onMount } from 'svelte';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import CpuIcon from '@lucide/svelte/icons/cpu';
  import SunIcon from '@lucide/svelte/icons/sun';
  import MoonIcon from '@lucide/svelte/icons/moon';
  import MonitorIcon from '@lucide/svelte/icons/monitor';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import CheckIcon from '@lucide/svelte/icons/check';
  import HardDriveIcon from '@lucide/svelte/icons/hard-drive';
  import PaletteIcon from '@lucide/svelte/icons/palette';
  import CloudIcon from '@lucide/svelte/icons/cloud';
  import PenLineIcon from '@lucide/svelte/icons/pen-line';
  import { MOD_KEY, isMac } from '$lib/keyboard.svelte.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { appState, THEME_PALETTES, STYLE_FLAVOURS } from '$lib/runes/app.svelte.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { resolve } from '$app/paths';
  import { llmManager } from '$lib/runes/llm.svelte.js';
  import { memoryManager } from '$lib/runes/memory.svelte.js';
  import { gitManager } from '$lib/runes/git.svelte.js';
  import { dropboxManager } from '$lib/runes/dropbox.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import GithubIcon from '@lucide/svelte/icons/git-branch';
  import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
  import DownloadCloudIcon from '@lucide/svelte/icons/download-cloud';
  import DropboxIcon from '@lucide/svelte/icons/box';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import { updaterManager } from '$lib/runes/updater.svelte.js';

  const SECTIONS = [
    { id: 'writing', label: 'Writing', icon: PenLineIcon },
    { id: 'appearance', label: 'Appearance', icon: PaletteIcon },
    { id: 'ai', label: 'AI', icon: CpuIcon },
    { id: 'sync', label: 'Cloud sync', icon: CloudIcon },
    { id: 'storage', label: 'Storage', icon: HardDriveIcon },
    { id: 'updates', label: 'Updates', icon: SparklesIcon },
  ];

  const PALETTE_DOT = {
    default: 'bg-[#7d5129]',
    zinc: 'bg-zinc-500',
    slate: 'bg-slate-500',
    rose: 'bg-rose-500',
    blue: 'bg-blue-500',
    green: 'bg-green-500',
    violet: 'bg-violet-500',
  };

  /** Text fields and popups in a row: an 8px fill well, like the sidebar search. */
  const FIELD =
    'h-7 min-w-0 rounded-field bg-fill px-2.5 text-sm text-foreground outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring/50';
  const NOTICE =
    'rounded-field px-3 py-2 text-sm wrap-anywhere max-h-40 overflow-y-auto overscroll-contain';

  const PARAGRAPH_STYLES = /** @type {const} */ ([
    { id: 'spaced', name: 'Spaced' },
    { id: 'indented', name: 'Indented' },
  ]);
  const THEMES = /** @type {const} */ ([
    { id: 'system', name: 'System', icon: MonitorIcon },
    { id: 'light', name: 'Light', icon: SunIcon },
    { id: 'dark', name: 'Dark', icon: MoonIcon },
  ]);
  const SYNC_PROVIDERS = [
    { id: 'none', name: 'This machine', icon: null },
    { id: 'github', name: 'GitHub', icon: GithubIcon },
    { id: 'dropbox', name: 'Dropbox', icon: DropboxIcon },
  ];

  const onMacOS = isMac();
  let activeSection = $state('writing');
  const activeLabel = $derived(SECTIONS.find((s) => s.id === activeSection)?.label ?? 'Settings');
  let repoInput = $state('');
  let commitMessage = $state('');
  let dropboxFolderInput = $state('');

  let openRouterKeyInput = $state('');

  onMount(() => {
    configManager.getConfig().then(() => {
      if (configManager.config?.ai_enabled) {
        memoryManager.setup();
        if (configManager.config?.provider === 'openrouter') {
          configManager.checkOpenRouterApiKey().then(() => {
            if (configManager.hasOpenRouterApiKey) configManager.fetchOpenRouterModels();
          });
        } else {
          llmManager.checkModels();
          llmManager.fetchSupportedModels();
        }
      }
    });
    gitManager.checkSession().then(() => {
      if (gitManager.user) gitManager.refreshStatus();
    });
    dropboxManager.checkSession().then(() => {
      if (dropboxManager.account && configManager.config?.dropbox_folder) {
        dropboxManager.refreshStatus();
      }
    });
  });

  async function handleToggleAiEnabled() {
    const enabling = !(configManager.config?.ai_enabled ?? false);
    await configManager.setAiEnabled(enabling);
    if (enabling) {
      await memoryManager.setup();
      memoryManager.reindexNotes();
      if (configManager.config?.provider === 'openrouter') {
        await configManager.checkOpenRouterApiKey();
        if (configManager.hasOpenRouterApiKey) await configManager.fetchOpenRouterModels();
      } else {
        await llmManager.setupModels();
        await llmManager.fetchSupportedModels();
      }
    }
  }

  async function handleSelectProvider(provider) {
    if (configManager.config?.provider === provider) return;
    await configManager.setAiProvider(provider);
    if (provider === 'openrouter') {
      await configManager.checkOpenRouterApiKey();
      if (configManager.hasOpenRouterApiKey) await configManager.fetchOpenRouterModels();
    } else {
      await llmManager.setupModels();
      await llmManager.fetchSupportedModels();
    }
  }

  async function handleSaveOpenRouterKey() {
    if (!openRouterKeyInput.trim()) return;
    await configManager.setOpenRouterApiKey(openRouterKeyInput.trim());
    openRouterKeyInput = '';
    await configManager.fetchOpenRouterModels();
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

  /** Preferred storage: 'none', 'github' or 'dropbox'. */
  const syncProvider = $derived(configManager.config?.sync_provider ?? 'none');

  async function handleSelectSyncProvider(provider) {
    if (syncProvider === provider) return;
    await configManager.setSyncProvider(provider);
    if (provider === 'github') {
      await gitManager.checkSession();
      if (gitManager.user) gitManager.refreshStatus();
    } else if (provider === 'dropbox') {
      await dropboxManager.checkSession();
      if (dropboxManager.account && configManager.config?.dropbox_folder) {
        dropboxManager.refreshStatus();
      }
    }
  }

  $effect(() => {
    if (configManager.config?.dropbox_folder && !dropboxFolderInput) {
      dropboxFolderInput = configManager.config.dropbox_folder;
    }
  });

  async function handleSaveDropboxFolder() {
    if (!dropboxFolderInput.trim()) return;
    await dropboxManager.setFolder(dropboxFolderInput.trim());
    await dropboxManager.refreshStatus();
  }

  async function handleDropboxPull() {
    if (!dropboxFolderInput.trim()) return;
    await dropboxManager.setFolder(dropboxFolderInput.trim());
    await dropboxManager.pull();
    if (dropboxManager.pullResult === 'success') {
      await fileManager.getRecents();
      await fileManager.getBinders();
    }
  }
</script>

{#snippet label(title, hint, forId = undefined)}
  <div class="flex-1 min-w-0">
    {#if forId}
      <label for={forId} class="block text-foreground">{title}</label>
    {:else}
      <p class="text-foreground">{title}</p>
    {/if}
    {#if hint}
      <p class="meta mt-0.5 leading-snug">{hint}</p>
    {/if}
  </div>
{/snippet}

{#snippet toggle(checked, onToggle, name)}
  <button
    type="button"
    role="switch"
    aria-label={name}
    aria-checked={checked}
    onclick={onToggle}
    class="relative inline-flex h-5.5 w-9 shrink-0 items-center rounded-full transition-colors
      {checked ? 'bg-primary' : 'bg-foreground/15'}"
  >
    <span
      class="inline-block size-4.5 rounded-full bg-white shadow-[0_1px_2px_rgb(0_0_0/0.25)] transition-transform
        {checked ? 'translate-x-4' : 'translate-x-0.5'}"
    ></span>
  </button>
{/snippet}

{#snippet spinner()}
  <span class="size-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"
  ></span>
{/snippet}

{#snippet paneNav(row)}
  {#each SECTIONS as section (section.id)}
    <button
      type="button"
      aria-current={activeSection === section.id ? 'page' : undefined}
      onclick={() => (activeSection = section.id)}
      class={row}
    >
      <section.icon strokeWidth={1.75} />
      <span>{section.label}</span>
    </button>
  {/each}
{/snippet}

<!-- System Settings layout: a sidebar of panes and one pane at a time on paper. -->
<div class="relative flex w-full h-full overflow-hidden gap-3 p-3">
  <aside
    aria-label="Settings"
    class="well hidden md:flex flex-col w-60 shrink-0 p-3 rounded-group h-full"
  >
    {#if onMacOS}
      <!-- The traffic lights sit here; the strip moves the window. -->
      <div
        data-tauri-drag-region="deep"
        class="h-[var(--titlebar-strip,28px)] shrink-0 -mt-3 -mx-3 mb-2"
      ></div>
    {/if}
    <a href={resolve('/')} class="sb-row text-muted-foreground">
      <ArrowLeftIcon strokeWidth={1.75} />
      Writings
    </a>
    <h2 class="sb-head">Settings</h2>
    <nav aria-label="Settings panes" class="flex flex-col gap-px">
      {@render paneNav('sb-row')}
    </nav>
  </aside>

  <main class="lifted flex-1 min-w-0 h-full flex flex-col rounded-group overflow-hidden">
    <header
      data-tauri-drag-region="deep"
      class="toolbar max-md:pl-[max(20px,var(--titlebar-inset-left,0px))]"
    >
      <a href={resolve('/')} class="tb-btn md:hidden -ml-2" aria-label="Back to writings">
        <ArrowLeftIcon strokeWidth={1.7} />
      </a>
      <h1 class="toolbar-title">{activeLabel}</h1>
    </header>

    <!-- Below 768px the panes wrap as chips instead of a sidebar, so every pane stays in sight. -->
    <nav aria-label="Settings panes" class="md:hidden flex flex-wrap gap-2 px-8 pb-3">
      {@render paneNav(
        'chip [&>svg]:size-3.5 aria-[current=page]:bg-foreground aria-[current=page]:text-paper'
      )}
    </nav>

    <ScrollFade class="flex-1 min-h-0">
      <ScrollArea type="scroll" class="w-full h-full">
        <div class="mx-auto max-w-160 flex flex-col px-8 pt-2 pb-16">
          {#if activeSection === 'writing'}
            <h2 class="group-head">Editor</h2>
            <div class="inset-group">
              <div class="inset-row">
                {@render label(
                  'Paragraph style',
                  'Spaced reads like a document; indented reads like a printed manuscript page, the style export uses for "Standard manuscript format".'
                )}
                <div class="seg flex shrink-0 gap-0.5 rounded-full bg-fill p-0.5">
                  {#each PARAGRAPH_STYLES as { id, name } (id)}
                    <button
                      type="button"
                      aria-pressed={writingState.paragraphStyle === id}
                      class="h-6 px-3 text-xs font-medium text-muted-foreground"
                      onclick={() => writingState.setParagraphStyle(id)}
                    >
                      {name}
                    </button>
                  {/each}
                </div>
              </div>
              <div class="inset-row">
                {@render label('Spellcheck', 'Underline misspelled words while you write.')}
                {@render toggle(
                  writingState.spellcheck,
                  () => writingState.setSpellcheck(!writingState.spellcheck),
                  'Spellcheck'
                )}
              </div>
              <div class="inset-row">
                {@render label(
                  'Smart punctuation',
                  'Curly quotes, em dashes from -- and ellipses from ... as you type. Backspace right after undoes one.'
                )}
                {@render toggle(
                  writingState.smartPunctuation,
                  () => writingState.setSmartPunctuation(!writingState.smartPunctuation),
                  'Smart punctuation'
                )}
              </div>
              <div class="inset-row">
                {@render label(
                  'Auto-close brackets',
                  'Typing an opening bracket adds its closing pair; with text selected, it wraps the selection.'
                )}
                {@render toggle(
                  writingState.autoPair,
                  () => writingState.setAutoPair(!writingState.autoPair),
                  'Auto-close brackets'
                )}
              </div>
              <div class="inset-row">
                {@render label(
                  'Highlight codex names',
                  `Dotted underline under characters, places and things from the binder's codex. Hover one for its summary, or ${MOD_KEY}-click to open its entry.`
                )}
                {@render toggle(
                  writingState.codexHighlight,
                  () => writingState.setCodexHighlight(!writingState.codexHighlight),
                  'Highlight codex names'
                )}
              </div>
            </div>

            <h2 class="group-head">Focus and goals</h2>
            <div class="inset-group">
              <div class="inset-row">
                {@render label(
                  'Focus mode goes full screen',
                  "Off, focus mode just hides the app's own chrome. On, it takes the whole screen, which on macOS moves the window to its own Space."
                )}
                {@render toggle(
                  writingState.focusFullscreen,
                  () => writingState.setFocusFullscreen(!writingState.focusFullscreen),
                  'Focus mode goes full screen'
                )}
              </div>
              <div class="inset-row">
                {@render label(
                  'Daily word goal',
                  'Shown next to the word count in the editor. Leave empty for no goal. Binder goals are set from the word count while writing.',
                  'settings-daily-goal'
                )}
                <input
                  id="settings-daily-goal"
                  type="number"
                  min="0"
                  step="100"
                  placeholder="No goal"
                  value={writingState.dailyGoal || ''}
                  onchange={(e) => writingState.setDailyGoal(Number(e.currentTarget.value))}
                  class="{FIELD} w-24 shrink-0 text-right tabular-nums"
                />
              </div>
            </div>
          {:else if activeSection === 'appearance'}
            <h2 class="group-head">Look</h2>
            <div class="inset-group">
              <div class="inset-row">
                {@render label('Theme', 'Light, dark, or follow your system setting.')}
                <div class="seg flex shrink-0 gap-0.5 rounded-full bg-fill p-0.5">
                  {#each THEMES as { id, name, icon: Icon } (id)}
                    <button
                      type="button"
                      aria-pressed={appState.ui.themePreference === id}
                      class="flex h-6 items-center gap-1.5 px-2.5 text-xs font-medium text-muted-foreground"
                      onclick={() => appState.setTheme(id)}
                    >
                      <Icon strokeWidth={1.75} class="size-3.5" />
                      {name}
                    </button>
                  {/each}
                </div>
              </div>
              <div class="inset-row">
                {@render label('Density', 'Text size and spacing of the writing.')}
                <div class="seg flex shrink-0 gap-0.5 rounded-full bg-fill p-0.5">
                  {#each [['default', 'Default'], ['compact', 'Compact']] as [id, name] (id)}
                    <button
                      type="button"
                      aria-pressed={appState.ui.density === id}
                      class="h-6 px-3 text-xs font-medium text-muted-foreground"
                      onclick={() => appState.setDensity(id)}
                    >
                      {name}
                    </button>
                  {/each}
                </div>
              </div>
              <div class="inset-row flex-wrap">
                {@render label('Accent colour')}
                <div class="flex flex-wrap gap-1.5" role="group" aria-label="Accent colour">
                  {#each THEME_PALETTES as palette (palette)}
                    <button
                      type="button"
                      title={palette}
                      aria-label={palette}
                      aria-pressed={appState.ui.themePalette === palette}
                      onclick={() => appState.setThemePalette(palette)}
                      class="size-5.5 rounded-full {PALETTE_DOT[palette] ??
                        'bg-muted-foreground'} ring-offset-2 ring-offset-card transition-shadow hover:ring-2 hover:ring-hair aria-pressed:ring-2 aria-pressed:ring-foreground/60"
                    ></button>
                  {/each}
                </div>
              </div>
            </div>

            <h2 class="group-head">Style</h2>
            <p class="meta -mt-1 mb-2 mx-3">
              How the page itself looks: typeface, spacing and the paper effect.
            </p>
            <div class="inset-group">
              {#each STYLE_FLAVOURS as flavour (flavour.id)}
                <button
                  type="button"
                  onclick={() => appState.setStyleFlavour(flavour.id)}
                  aria-pressed={appState.ui.styleFlavour === flavour.id}
                  class="inset-row w-full text-left hover:bg-fill aria-pressed:bg-selection"
                >
                  {@render label(flavour.label, flavour.description)}
                  {#if appState.ui.styleFlavour === flavour.id}
                    <CheckIcon strokeWidth={2} class="size-4 shrink-0 text-primary" />
                  {/if}
                </button>
              {/each}
            </div>
          {:else if activeSection === 'storage'}
            <h2 class="group-head">On this machine</h2>
            <div class="inset-group">
              <div class="inset-row items-start">
                <FolderIcon strokeWidth={1.75} class="size-4 mt-0.5 shrink-0 text-primary" />
                <div class="flex-1 min-w-0">
                  <p>Content directory</p>
                  <p class="meta mt-0.5">Where your writings are stored as Markdown files.</p>
                  <code class="mt-1.5 block text-xs break-all font-mono text-foreground/80">
                    {configManager.config?.content_directory || 'Loading…'}
                  </code>
                </div>
              </div>
              <div class="inset-row items-start">
                <FolderIcon strokeWidth={1.75} class="size-4 mt-0.5 shrink-0 text-primary" />
                <div class="flex-1 min-w-0">
                  <p>History directory</p>
                  <p class="meta mt-0.5">Where version history for each writing is kept.</p>
                  <code class="mt-1.5 block text-xs break-all font-mono text-foreground/80">
                    {configManager.config?.undotree_dir || 'Loading…'}
                  </code>
                </div>
              </div>
            </div>
          {:else if activeSection === 'ai'}
            <h2 class="group-head">Assistant</h2>
            <div class="inset-group">
              <div class="inset-row">
                {@render label(
                  'Enable AI',
                  'Turns on AI chat, autocomplete, and model downloads. Off by default so Memoire starts as a plain writing app.'
                )}
                {@render toggle(
                  configManager.config?.ai_enabled ?? false,
                  handleToggleAiEnabled,
                  'Enable AI'
                )}
              </div>
              {#if configManager.config?.ai_enabled}
                <div class="inset-row">
                  {@render label(
                    'Provider',
                    'Whether chat, autocomplete, and grammar checks run on this machine or through OpenRouter. Note search always stays local.'
                  )}
                  <div class="seg flex shrink-0 gap-0.5 rounded-full bg-fill p-0.5">
                    {#each [['local', 'Local'], ['openrouter', 'OpenRouter']] as [id, name] (id)}
                      <button
                        type="button"
                        aria-pressed={(configManager.config?.provider ?? 'local') === id}
                        class="h-6 px-3 text-xs font-medium text-muted-foreground"
                        onclick={() => handleSelectProvider(id)}
                      >
                        {name}
                      </button>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>

            {#if configManager.config?.ai_enabled}
              {#if configManager.config?.provider === 'openrouter'}
                <h2 class="group-head">OpenRouter</h2>
                <div class="inset-group">
                  {#if configManager.hasOpenRouterApiKey}
                    <div class="inset-row">
                      {@render label('API key', 'Saved on this machine.')}
                      <Button
                        variant="ghost"
                        size="sm"
                        onclick={() => configManager.clearOpenRouterApiKey()}
                      >
                        Remove key
                      </Button>
                    </div>
                    <div class="inset-row">
                      {#if configManager.isLoadingOpenRouterModels}
                        {@render label('Model')}
                        <span class="meta flex items-center gap-2"
                          >{@render spinner()} Loading models…</span
                        >
                      {:else if configManager.openRouterModels.length > 0}
                        {@render label('Model', undefined, 'settings-openrouter-model')}
                        <select
                          id="settings-openrouter-model"
                          class="{FIELD} w-56 max-w-[50%]"
                          value={configManager.config?.openrouter_model ??
                            'meta-llama/llama-3.1-8b-instruct:free'}
                          onchange={(e) => configManager.setOpenRouterModel(e.currentTarget.value)}
                        >
                          {#each configManager.openRouterModels as m (m.id)}
                            <option value={m.id}>{m.name}</option>
                          {/each}
                        </select>
                      {:else}
                        {@render label(
                          'Model',
                          configManager.config?.openrouter_model ??
                            'meta-llama/llama-3.1-8b-instruct:free'
                        )}
                        <Button
                          variant="secondary"
                          size="sm"
                          onclick={() => configManager.fetchOpenRouterModels()}
                        >
                          Load model list
                        </Button>
                      {/if}
                    </div>
                  {:else}
                    <div class="inset-row flex-col items-stretch gap-2">
                      {@render label(
                        'API key',
                        'Enter an OpenRouter API key to use remote models for chat, autocomplete, and grammar checks.'
                      )}
                      <div class="flex gap-2">
                        <input
                          type="password"
                          aria-label="OpenRouter API key"
                          class="{FIELD} flex-1"
                          placeholder="sk-or-..."
                          bind:value={openRouterKeyInput}
                        />
                        <Button
                          size="sm"
                          onclick={handleSaveOpenRouterKey}
                          disabled={!openRouterKeyInput.trim()}
                        >
                          Save
                        </Button>
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}

              {#if (configManager.config?.provider ?? 'local') === 'local'}
                {@const defaultId =
                  configManager.config?.default_llm_model_id ?? 'qwen_2_5_1_5b_instruct'}
                <h2 class="group-head">Local models</h2>
                {#if llmManager.error}
                  <div class="{NOTICE} mb-2 bg-destructive/10 text-destructive">
                    {llmManager.error}
                  </div>
                {/if}
                <div class="inset-group">
                  {#if llmManager.supportedModels.length > 0}
                    {#each llmManager.supportedModels as model (model.id)}
                      <div class="inset-row">
                        <div class="flex-1 min-w-0">
                          <p class="truncate">{model.name}</p>
                          <p class="meta mt-0.5 capitalize">{model.model_type}</p>
                        </div>
                        {#if model.downloaded}
                          {#if model.id === defaultId}
                            <span class="meta flex items-center gap-1">
                              <CheckIcon strokeWidth={2} class="size-3.5 text-primary" />
                              Default
                            </span>
                          {:else}
                            <span class="meta">Downloaded</span>
                            <Button
                              variant="ghost"
                              size="sm"
                              onclick={() => configManager.setDefaultLlmModel(model.id)}
                            >
                              Set as default
                            </Button>
                          {/if}
                        {:else}
                          {@const isDownloading = llmManager.downloadingModelId === model.id}
                          <Button
                            variant="secondary"
                            size="sm"
                            onclick={() => llmManager.downloadModel(model.id)}
                            disabled={isDownloading || !!llmManager.downloadingModelId}
                          >
                            {#if isDownloading}
                              {@render spinner()}
                              Downloading {llmManager.loadingProgress}%
                            {:else}
                              Download
                            {/if}
                          </Button>
                        {/if}
                      </div>
                    {/each}
                  {:else}
                    <div class="inset-row meta">{@render spinner()} Loading supported models…</div>
                  {/if}
                </div>
                <p class="meta mt-2 mx-3">
                  Models are stored in your app directory. Only downloaded models can be set as
                  default.
                </p>
              {/if}

              <h2 class="group-head">Note search</h2>
              <div class="inset-group">
                <div class="inset-row">
                  <div class="flex-1 min-w-0">
                    <p>
                      {#if memoryManager.indexProgress}
                        {memoryManager.indexProgress.total
                          ? `Indexing ${memoryManager.indexProgress.done} of ${memoryManager.indexProgress.total} notes…`
                          : 'Preparing to index…'}
                      {:else if memoryManager.indexStatus}
                        {memoryManager.indexStatus.documents} notes · {memoryManager.indexStatus
                          .passages} passages indexed
                      {:else}
                        Checking index…
                      {/if}
                    </p>
                    <p class="meta mt-0.5 leading-snug">
                      Notes are indexed on this device so the assistant can search them. Saved
                      changes are picked up automatically; reindex after importing notes from
                      elsewhere.
                    </p>
                    {#if memoryManager.indexProgress?.total}
                      <div class="mt-2 h-1 w-full rounded-full bg-fill overflow-hidden">
                        <div
                          class="h-full w-full origin-left bg-primary transition-transform duration-200 ease-out"
                          style="transform: scaleX({memoryManager.indexProgress.done /
                            memoryManager.indexProgress.total})"
                        ></div>
                      </div>
                    {/if}
                  </div>
                  <Button
                    variant="secondary"
                    size="sm"
                    disabled={!!memoryManager.indexProgress}
                    onclick={() => memoryManager.reindexNotes(true)}
                  >
                    Reindex notes
                  </Button>
                </div>
              </div>

              <h2 class="group-head">System prompt</h2>
              <div class="inset-group">
                <textarea
                  aria-label="System prompt"
                  class="block min-h-30 w-full resize-y bg-transparent px-5 py-3.5 text-sm outline-none placeholder:text-muted-foreground focus-visible:bg-fill"
                  placeholder="e.g. You are a helpful AI assistant..."
                  value={configManager.config?.system_prompt ?? ''}
                  onchange={(e) => configManager.setSystemPrompt(e.currentTarget.value)}></textarea>
              </div>
              <p class="meta mt-2 mx-3">
                The instructions that guide the assistant in the chat panel.
              </p>
            {/if}
          {:else if activeSection === 'sync'}
            <h2 class="group-head">Sync</h2>
            <div class="inset-group">
              <div class="inset-row flex-wrap">
                {@render label(
                  'Preferred storage',
                  'One backend is active at a time; the other keeps its settings and stays idle.'
                )}
                <div class="seg flex shrink-0 gap-0.5 rounded-full bg-fill p-0.5">
                  {#each SYNC_PROVIDERS as { id, name, icon: Icon } (id)}
                    <button
                      type="button"
                      aria-pressed={syncProvider === id}
                      class="flex h-6 items-center gap-1.5 px-2.5 text-xs font-medium text-muted-foreground"
                      onclick={() => handleSelectSyncProvider(id)}
                    >
                      {#if Icon}<Icon strokeWidth={1.75} class="size-3.5" />{/if}
                      {name}
                    </button>
                  {/each}
                </div>
              </div>
              {#if syncProvider === 'none'}
                <div class="inset-row meta leading-snug">
                  Nothing leaves your machine. Pick GitHub or Dropbox to sync your writing;
                  whichever you've already set up stays configured.
                </div>
              {/if}
            </div>

            {#if syncProvider === 'github'}
              <h2 class="group-head">GitHub account</h2>
              {#if gitManager.error}
                <div class="{NOTICE} mb-2 bg-destructive/10 text-destructive">
                  {typeof gitManager.error === 'string'
                    ? gitManager.error
                    : 'Something went wrong.'}
                </div>
              {/if}
              <div class="inset-group">
                {#if gitManager.isCheckingSession}
                  <div class="inset-row meta">{@render spinner()} Checking session…</div>
                {:else if gitManager.user}
                  <div class="inset-row">
                    {#if gitManager.user.avatar_url}
                      <img src={gitManager.user.avatar_url} alt="" class="size-8 rounded-full" />
                    {/if}
                    <div class="flex-1 min-w-0">
                      <p class="font-medium truncate">
                        {gitManager.user.name || gitManager.user.login}
                      </p>
                      <p class="meta truncate">@{gitManager.user.login}</p>
                    </div>
                    <Button variant="secondary" size="sm" onclick={() => gitManager.logout()}
                      >Disconnect</Button
                    >
                  </div>
                  <div class="inset-row">
                    {@render label(
                      'Auto-push on exit',
                      'Commit and push any pending changes to GitHub when you close Memoire.'
                    )}
                    {@render toggle(
                      configManager.config?.auto_push_on_exit ?? false,
                      () =>
                        configManager.setAutoPushOnExit(
                          !(configManager.config?.auto_push_on_exit ?? false)
                        ),
                      'Auto-push on exit'
                    )}
                  </div>
                {:else if gitManager.deviceCode}
                  <div class="inset-row flex-col items-start gap-2">
                    <p class="meta">
                      Enter this code at
                      <span class="font-mono text-primary"
                        >{gitManager.deviceCode.verification_uri}</span
                      >
                      (opened in your browser):
                    </p>
                    <span class="text-2xl font-mono tracking-widest font-medium"
                      >{gitManager.deviceCode.user_code}</span
                    >
                    <div class="flex w-full items-center gap-2 meta">
                      {@render spinner()} Waiting for approval…
                      <Button
                        variant="ghost"
                        size="sm"
                        class="ml-auto"
                        onclick={() => gitManager.cancelLogin()}>Cancel</Button
                      >
                    </div>
                  </div>
                {:else}
                  <div class="inset-row">
                    {@render label(
                      'Not connected',
                      'Sign in to sync your content directory with a GitHub repository.'
                    )}
                    <Button
                      variant="secondary"
                      size="sm"
                      onclick={() => gitManager.startLogin()}
                      disabled={gitManager.isConnecting}
                    >
                      <GithubIcon strokeWidth={1.75} />
                      Login with GitHub
                    </Button>
                  </div>
                {/if}
              </div>

              {#if gitManager.user}
                <h2 class="group-head">Repository</h2>
                <div class="inset-group">
                  <div class="inset-row flex-col items-stretch gap-2">
                    {@render label(
                      'owner/repo',
                      'Writing in your content directory is committed and pushed here. Already have writing on GitHub? Import pulls it into this content directory.'
                    )}
                    <div class="flex flex-wrap items-center gap-2">
                      <input
                        type="text"
                        aria-label="GitHub repository"
                        bind:value={repoInput}
                        placeholder="owner/repo"
                        class="{FIELD} flex-1 basis-40"
                      />
                      <Button variant="secondary" size="sm" onclick={handleSaveRepo}>Save</Button>
                      <Button
                        variant="secondary"
                        size="sm"
                        onclick={handleImport}
                        disabled={gitManager.isImporting || !repoInput.trim()}
                      >
                        {#if gitManager.isImporting}
                          {@render spinner()} Importing…
                        {:else}
                          <DownloadCloudIcon strokeWidth={1.75} /> Import
                        {/if}
                      </Button>
                    </div>
                    {#if gitManager.importResult === 'success'}
                      <div class="{NOTICE} bg-success/10 text-success">Imported from GitHub.</div>
                    {:else if gitManager.importResult === 'error'}
                      <div class="{NOTICE} bg-destructive/10 text-destructive">
                        Import failed: {typeof gitManager.error === 'string'
                          ? gitManager.error
                          : 'unknown error'}
                      </div>
                    {/if}
                  </div>
                </div>

                <div class="flex items-end">
                  <h2 class="group-head flex-1">Changes</h2>
                  <Button
                    variant="ghost"
                    size="sm"
                    class="mb-1"
                    onclick={() => gitManager.refreshStatus()}>Refresh</Button
                  >
                </div>
                <div class="inset-group">
                  {#if gitManager.isLoadingStatus}
                    <div class="inset-row meta">{@render spinner()} Loading status…</div>
                  {:else if gitManager.status.length === 0}
                    <div class="inset-row meta">No changes to commit.</div>
                  {:else}
                    <div class="max-h-40 overflow-y-auto">
                      {#each gitManager.status as file (file.path)}
                        <div class="flex items-center justify-between gap-3 px-5 py-1.5 text-xs">
                          <span class="font-mono truncate">{file.path}</span>
                          <span class="meta shrink-0">{file.status}</span>
                        </div>
                      {/each}
                    </div>
                  {/if}
                  <textarea
                    aria-label="Commit message"
                    class="block min-h-20 w-full resize-y bg-transparent px-5 py-3.5 text-sm outline-none placeholder:text-muted-foreground focus-visible:bg-fill"
                    placeholder="Commit message"
                    bind:value={commitMessage}></textarea>
                  <div class="inset-row justify-end">
                    {#if gitManager.pushResult === 'success'}
                      <span class="meta text-success! mr-auto">Pushed to GitHub.</span>
                    {/if}
                    <Button
                      size="sm"
                      onclick={handleCommitPush}
                      disabled={gitManager.isPushing || !commitMessage.trim()}
                    >
                      {#if gitManager.isPushing}
                        {@render spinner()} Pushing…
                      {:else}
                        <UploadCloudIcon strokeWidth={1.75} /> Commit & Push
                      {/if}
                    </Button>
                  </div>
                </div>
              {/if}
            {:else if syncProvider === 'dropbox'}
              <h2 class="group-head">Dropbox account</h2>
              {#if dropboxManager.error}
                <div class="{NOTICE} mb-2 bg-destructive/10 text-destructive">
                  {typeof dropboxManager.error === 'string'
                    ? dropboxManager.error
                    : 'Something went wrong.'}
                </div>
              {/if}
              <div class="inset-group">
                {#if dropboxManager.isCheckingSession}
                  <div class="inset-row meta">{@render spinner()} Checking session…</div>
                {:else if dropboxManager.account}
                  <div class="inset-row">
                    {#if dropboxManager.account.photo_url}
                      <img
                        src={dropboxManager.account.photo_url}
                        alt=""
                        class="size-8 rounded-full"
                      />
                    {/if}
                    <div class="flex-1 min-w-0">
                      <p class="font-medium truncate">{dropboxManager.account.name}</p>
                      {#if dropboxManager.account.email}
                        <p class="meta truncate">{dropboxManager.account.email}</p>
                      {/if}
                    </div>
                    <Button variant="secondary" size="sm" onclick={() => dropboxManager.logout()}
                      >Disconnect</Button
                    >
                  </div>
                  <div class="inset-row">
                    {@render label(
                      'Auto-push on exit',
                      'Upload any changed writing to Dropbox when you close Memoire.'
                    )}
                    {@render toggle(
                      configManager.config?.dropbox_auto_push_on_exit ?? false,
                      () =>
                        configManager.setDropboxAutoPushOnExit(
                          !(configManager.config?.dropbox_auto_push_on_exit ?? false)
                        ),
                      'Auto-push to Dropbox on exit'
                    )}
                  </div>
                {:else if dropboxManager.isConnecting}
                  <div class="inset-row">
                    <div class="flex-1 min-w-0">
                      <p>Approve Memoire in the Dropbox page that opened in your browser.</p>
                      <p class="meta mt-0.5 flex items-center gap-2">
                        {@render spinner()} Waiting for approval…
                      </p>
                    </div>
                    <Button variant="ghost" size="sm" onclick={() => dropboxManager.cancelLogin()}
                      >Cancel</Button
                    >
                  </div>
                {:else}
                  <div class="inset-row">
                    {@render label(
                      'Not connected',
                      'Connect to sync your content directory with a Dropbox folder.'
                    )}
                    <Button
                      variant="secondary"
                      size="sm"
                      onclick={() => dropboxManager.startLogin()}
                    >
                      <DropboxIcon strokeWidth={1.75} />
                      Connect Dropbox
                    </Button>
                  </div>
                {/if}
              </div>

              {#if dropboxManager.account}
                <h2 class="group-head">Folder</h2>
                <div class="inset-group">
                  <div class="inset-row flex-col items-stretch gap-2">
                    {@render label(
                      'Dropbox folder',
                      "Writing in your content directory is uploaded here; the folder is created if it doesn't exist yet. Pull brings writing from Dropbox down into this content directory."
                    )}
                    <div class="flex flex-wrap items-center gap-2">
                      <input
                        type="text"
                        aria-label="Dropbox folder"
                        bind:value={dropboxFolderInput}
                        placeholder="/Memoire"
                        class="{FIELD} flex-1 basis-40"
                      />
                      <Button variant="secondary" size="sm" onclick={handleSaveDropboxFolder}
                        >Save</Button
                      >
                      <Button
                        variant="secondary"
                        size="sm"
                        onclick={handleDropboxPull}
                        disabled={dropboxManager.isPulling || !dropboxFolderInput.trim()}
                      >
                        {#if dropboxManager.isPulling}
                          {@render spinner()} Pulling…
                        {:else}
                          <DownloadCloudIcon strokeWidth={1.75} /> Pull
                        {/if}
                      </Button>
                    </div>
                    {#if dropboxManager.pullResult === 'success'}
                      <div class="{NOTICE} bg-success/10 text-success">
                        Pulled {dropboxManager.lastSync?.downloaded ?? 0} file(s) from Dropbox.
                        {#if dropboxManager.lastSync?.conflicts}
                          {dropboxManager.lastSync.conflicts} file(s) changed on both sides — Dropbox's
                          version was saved beside yours as "(Dropbox conflict)".
                        {/if}
                      </div>
                    {:else if dropboxManager.pullResult === 'error'}
                      <div class="{NOTICE} bg-destructive/10 text-destructive">
                        Pull failed: {typeof dropboxManager.error === 'string'
                          ? dropboxManager.error
                          : 'unknown error'}
                      </div>
                    {/if}
                  </div>
                </div>

                <div class="flex items-end">
                  <h2 class="group-head flex-1">Changes</h2>
                  <Button
                    variant="ghost"
                    size="sm"
                    class="mb-1"
                    onclick={() => dropboxManager.refreshStatus()}>Refresh</Button
                  >
                </div>
                <div class="inset-group">
                  {#if dropboxManager.isLoadingStatus}
                    <div class="inset-row meta">{@render spinner()} Loading status…</div>
                  {:else if dropboxManager.status.length === 0}
                    <div class="inset-row meta">Everything is in sync.</div>
                  {:else}
                    <div class="max-h-40 overflow-y-auto">
                      {#each dropboxManager.status as file (file.path)}
                        <div class="flex items-center justify-between gap-3 px-5 py-1.5 text-xs">
                          <span class="font-mono truncate">{file.path}</span>
                          <span
                            class="meta shrink-0 {file.status === 'conflict'
                              ? 'text-destructive!'
                              : ''}">{file.status.replace('_', ' ')}</span
                          >
                        </div>
                      {/each}
                    </div>
                  {/if}
                  <div class="inset-row justify-end">
                    {#if dropboxManager.pushResult === 'success'}
                      <span class="meta text-success! mr-auto">
                        Pushed {dropboxManager.lastSync?.uploaded ?? 0} file(s) to Dropbox.
                        {#if dropboxManager.lastSync?.conflicts}
                          Skipped {dropboxManager.lastSync.conflicts} file(s) changed on both sides —
                          pull to keep both versions.
                        {/if}
                      </span>
                    {/if}
                    <Button
                      size="sm"
                      onclick={() => dropboxManager.push()}
                      disabled={dropboxManager.isPushing || !configManager.config?.dropbox_folder}
                    >
                      {#if dropboxManager.isPushing}
                        {@render spinner()} Pushing…
                      {:else}
                        <UploadCloudIcon strokeWidth={1.75} /> Push to Dropbox
                      {/if}
                    </Button>
                  </div>
                </div>
              {/if}
            {/if}
          {:else if activeSection === 'updates'}
            <h2 class="group-head">Software update</h2>
            <div class="inset-group">
              <div class="inset-row">
                {@render label(
                  `Memoire ${updaterManager.currentVersion ?? ''}`,
                  'Memoire checks for new versions in the background and downloads them quietly. Nothing installs until you restart.'
                )}
                {#if updaterManager.status === 'ready'}
                  <Button size="sm" onclick={() => updaterManager.installAndRestart()}>
                    <RefreshCwIcon strokeWidth={1.75} />
                    Restart to update
                  </Button>
                {:else}
                  <Button
                    variant="secondary"
                    size="sm"
                    onclick={() => updaterManager.check()}
                    disabled={updaterManager.isBusy}
                  >
                    {#if updaterManager.status === 'checking'}
                      {@render spinner()} Checking…
                    {:else if updaterManager.status === 'downloading'}
                      {@render spinner()} Downloading…
                    {:else if updaterManager.status === 'installing'}
                      {@render spinner()} Installing…
                    {:else}
                      <RefreshCwIcon strokeWidth={1.75} /> Check for updates
                    {/if}
                  </Button>
                {/if}
              </div>

              {#if updaterManager.status === 'downloading' && updaterManager.progress !== null}
                <div class="inset-row">
                  <div class="h-1.5 w-full rounded-full bg-fill overflow-hidden">
                    <div
                      class="h-full w-full origin-left bg-primary transition-transform duration-200 ease-out"
                      style="transform: scaleX({updaterManager.progress})"
                    ></div>
                  </div>
                </div>
              {/if}

              {#if updaterManager.available && ['downloading', 'ready', 'installing'].includes(updaterManager.status)}
                <div class="inset-row flex-col items-start gap-1">
                  <p class="font-medium">Version {updaterManager.available.version}</p>
                  {#if updaterManager.available.notes}
                    <p class="meta whitespace-pre-line max-h-40 overflow-y-auto">
                      {updaterManager.available.notes}
                    </p>
                  {/if}
                </div>
              {:else if updaterManager.status === 'up-to-date'}
                <div class="inset-row meta">You're on the latest version.</div>
              {:else if updaterManager.status === 'error'}
                <div class="inset-row text-destructive wrap-anywhere">
                  {updaterManager.error ?? 'Update check failed.'}
                </div>
              {/if}
            </div>
            {#if updaterManager.lastChecked}
              <p class="meta mt-2 mx-3">
                Last checked {updaterManager.lastChecked.toLocaleString()}
              </p>
            {/if}
          {/if}
        </div>
      </ScrollArea>
    </ScrollFade>
  </main>
</div>
