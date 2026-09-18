<script>
  import '../app.css';
  import Cmdk from '$lib/components/cmdk.svelte';
  import ShortcutsHelp from '$lib/components/shortcuts-help.svelte';
  import NewWritingPicker from '$lib/components/new-writing-picker.svelte';
  import ExportDialog from '$lib/components/export-dialog.svelte';
  import ProjectSearch from '$lib/components/project-search.svelte';
  import ImportDialog from '$lib/components/import-dialog.svelte';
  import { page } from '$app/state';
  import { startNewWriting } from '$lib/new-writing.js';
  import { dirOf } from '$lib/runes/fs.svelte.js';

  import { appState, THEME_PALETTES, DENSITY_MODES, STYLE_FLAVOURS } from '@/runes/app.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { onDestroy, onMount } from 'svelte';
  import { isMod, isMac } from '$lib/keyboard.svelte.js';
  import { Toaster } from 'svelte-sonner';

  import { TooltipProvider } from '$lib/components/ui/tooltip/index.js';

  let { children } = $props();

  const onMacOS = isMac();

  /**
   * ⌘N: a new writing where the writer already is. In the editor that is the
   * open document's folder (next scene, next chapter); on the home screen the
   * selected binder; anywhere else the top level.
   */
  function newWritingHere() {
    const current = page.params.lexical;
    if (current) {
      startNewWriting(dirOf(current));
    } else {
      startNewWriting(appState.ui.activeBinder ?? '');
    }
  }

  onMount(() => {
    const savedTheme = localStorage.getItem('theme');

    if (savedTheme === 'light' || savedTheme === 'dark' || savedTheme === 'system') {
      appState.setTheme(savedTheme);
    } else {
      appState.setTheme('system');
    }

    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const onSystemThemeChange = () => appState.syncSystemTheme();
    media.addEventListener('change', onSystemThemeChange);

    const savedPalette = localStorage.getItem('themePalette');

    if (savedPalette && THEME_PALETTES.some((p) => p === savedPalette)) {
      appState.setThemePalette(savedPalette);
    }
    const savedDensity = localStorage.getItem('density');
    if (savedDensity && DENSITY_MODES.some((d) => d === savedDensity)) {
      appState.setDensity(savedDensity);
    }
    const savedFlavour = localStorage.getItem('styleFlavour');
    if (savedFlavour && STYLE_FLAVOURS.some((f) => f.id === savedFlavour)) {
      appState.setStyleFlavour(savedFlavour);
    }
    const savedFontSize = localStorage.getItem('fontSize');
    if (savedFontSize) {
      const size = parseInt(savedFontSize);
      if (!isNaN(size)) {
        appState.setFontSize(size);
      }
    }
    writingState.load();
    if (localStorage.getItem('outlineOpen') === 'true') {
      appState.toggleOutline(true);
    }
    fileManager.getRecents();
    fileManager.getBinders();
    fileManager.getFolders();

    configManager.getConfig().then(() => {
      if (configManager.config?.ai_enabled) {
        llmManager.setupModels();
        // Picks up notes changed outside the app and prunes deleted ones; unchanged
        // paragraphs are skipped, so this stays quick after the first run.
        memoryManager.setup().then(() => memoryManager.reindexNotes());
        if (configManager.config?.provider === 'openrouter') {
          configManager.checkOpenRouterApiKey();
        }
      }
    });

    return () => media.removeEventListener('change', onSystemThemeChange);
  });

  $effect(() => {
    if (typeof document !== 'undefined') {
      const html = document.documentElement;
      if (appState.ui.theme === 'dark') {
        html.classList.add('dark');
        document.body.classList.add('dark');
      } else {
        html.classList.remove('dark');
        document.body.classList.remove('dark');
      }
    }
  });

  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.style.fontSize = appState.ui.density === 'compact' ? '15px' : '18px';
    }
  });

  onDestroy(() => {
    llmManager.destroy();
  });
</script>

<svelte:body />

<TooltipProvider>
  <div
    class="{appState.ui.theme} {appState.ui.themePalette !== 'default'
      ? `theme-${appState.ui.themePalette}`
      : ''} style-{appState.ui.styleFlavour} density-{appState.ui.density} {onMacOS
      ? 'platform-mac'
      : ''} font-sans font-normal w-full h-screen bg-background text-foreground overflow-hidden relative"
  >
    {#if onMacOS}
      <!-- Overlay title bar: this strip is what the user grabs to move the window. -->
      <div
        data-tauri-drag-region
        class="fixed inset-x-0 top-0 z-10 h-(--titlebar-height) bg-titlebar-background cursor-default select-none"
      ></div>
    {/if}
    {@render children()}
    <Cmdk />
    <NewWritingPicker />
    <ExportDialog />
    <ProjectSearch />
    <ImportDialog />
    <ShortcutsHelp />
    <Toaster
      theme={appState.ui.theme}
      position="bottom-right"
      offset={56}
      closeButton
      toastOptions={{
        class:
          'font-sans !bg-popover !text-popover-foreground !border-border !shadow-lg !rounded-xl',
        descriptionClass: '!text-muted-foreground',
      }}
    />
  </div>
</TooltipProvider>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Backspace') {
      const target = e.target;
      const isInput =
        target instanceof HTMLElement &&
        (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);

      if (!isInput) {
        e.preventDefault();
      }
    }

    if (e.key === 'k' && isMod(e)) {
      e.preventDefault();
      appState.toggleCommandMenu(!appState.ui.isCommandMenuOpen);
    }

    if (e.key.toLowerCase() === 'n' && isMod(e) && !e.altKey) {
      e.preventDefault();
      if (e.shiftKey) {
        appState.toggleNewPicker(true);
      } else {
        newWritingHere();
      }
    }

    if (e.key.toLowerCase() === 'f' && isMod(e) && e.shiftKey && !e.altKey) {
      e.preventDefault();
      appState.toggleProjectSearch(true);
    }

    if (e.key.toLowerCase() === 'p' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      appState.toggleCommandMenu(!appState.ui.isCommandMenuOpen);
    }

    if (isMod(e)) {
      if (e.key === '=' || e.key === '+') {
        e.preventDefault();
        appState.increaseFontSize();
      } else if (e.key === '-') {
        e.preventDefault();
        appState.decreaseFontSize();
      }
    }
  }}
/>

<style>
  /**
   * Source Serif 4 — display/body serif (variable weight)
   */
  @font-face {
    font-family: 'Source Serif 4';
    src: url('/fonts/SourceSerif4/source-serif4-normal.woff2') format('woff2');
    font-weight: 200 900;
    font-style: normal;
    font-display: swap;
  }
  @font-face {
    font-family: 'Source Serif 4';
    src: url('/fonts/SourceSerif4/source-serif4-italic.woff2') format('woff2');
    font-weight: 200 900;
    font-style: italic;
    font-display: swap;
  }

  /**
   * Public Sans — UI/chrome sans (variable weight)
   */
  @font-face {
    font-family: 'Public Sans';
    src: url('/fonts/PublicSans/public-sans.woff2') format('woff2');
    font-weight: 100 900;
    font-style: normal;
    font-display: swap;
  }

  /**
   * JetBrains Mono — metadata/mono (variable weight)
   */
  @font-face {
    font-family: 'JetBrains Mono';
    src: url('/fonts/JetBrainsMono/jetbrains-mono.woff2') format('woff2');
    font-weight: 100 800;
    font-style: normal;
    font-display: swap;
  }

  /**
   * Playfair Display — Paper style headings only (variable weight)
   */
  @font-face {
    font-family: 'Playfair Display';
    src: url('/fonts/PlayfairDisplay/playfair-display.woff2') format('woff2');
    font-weight: 400 900;
    font-style: normal;
    font-display: swap;
  }
</style>
