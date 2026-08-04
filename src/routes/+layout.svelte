<script>
  import '../app.css';
  import Cmdk from '$lib/components/cmdk.svelte';
  import ShortcutsHelp from '$lib/components/shortcuts-help.svelte';

  import { appState, THEME_PALETTES, DENSITY_MODES } from '@/runes/app.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { onDestroy, onMount } from 'svelte';
  import { isMod } from '$lib/keyboard.svelte.js';

  import { TooltipProvider } from '$lib/components/ui/tooltip/index.js';

  let { children } = $props();

  onMount(() => {
    const savedTheme = localStorage.getItem('theme');

    if (savedTheme === 'light' || savedTheme === 'dark') {
      appState.setTheme(savedTheme);
    } else {
      appState.setTheme('dark');
    }

    const savedPalette = localStorage.getItem('themePalette');

    if (savedPalette && THEME_PALETTES.includes(savedPalette)) {
      appState.setThemePalette(savedPalette);
    }
    const savedDensity = localStorage.getItem('density');
    if (savedDensity && DENSITY_MODES.includes(savedDensity)) {
      appState.setDensity(savedDensity);
    }
    const savedFontSize = localStorage.getItem('fontSize');
    if (savedFontSize) {
      const size = parseInt(savedFontSize);
      if (!isNaN(size)) {
        appState.setFontSize(size);
      }
    }
    fileManager.getRecents();
    fileManager.getBinders();
    llmManager.setupModels();
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
      : ''} style-{appState.ui.styleFlavour} density-{appState.ui
      .density} font-writer font-normal w-full h-screen bg-background text-foreground overflow-hidden relative"
  >
    {@render children()}
    <Cmdk />
    <ShortcutsHelp />
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
     * EB Garamond Font Family
     */

  /* Regular */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-Regular.ttf') format('truetype');
    font-weight: 400;
    font-style: normal;
    font-display: swap;
  }

  /* Italic */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-Italic.ttf') format('truetype');
    font-weight: 400;
    font-style: italic;
    font-display: swap;
  }

  /* Medium */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-Medium.ttf') format('truetype');
    font-weight: 500;
    font-style: normal;
    font-display: swap;
  }

  /* Medium Italic */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-MediumItalic.ttf') format('truetype');
    font-weight: 500;
    font-style: italic;
    font-display: swap;
  }

  /* SemiBold */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-SemiBold.ttf') format('truetype');
    font-weight: 600;
    font-style: normal;
    font-display: swap;
  }

  /* SemiBold Italic */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-SemiBoldItalic.ttf') format('truetype');
    font-weight: 600;
    font-style: italic;
    font-display: swap;
  }

  /* Bold */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-Bold.ttf') format('truetype');
    font-weight: 700;
    font-style: normal;
    font-display: swap;
  }

  /* Bold Italic */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-BoldItalic.ttf') format('truetype');
    font-weight: 700;
    font-style: italic;
    font-display: swap;
  }

  /* ExtraBold */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-ExtraBold.ttf') format('truetype');
    font-weight: 800;
    font-style: normal;
    font-display: swap;
  }

  /* ExtraBold Italic */
  @font-face {
    font-family: 'EB Garamond';
    src: url('/fonts/EBGaramond/EBGaramond-ExtraBoldItalic.ttf') format('truetype');
    font-weight: 800;
    font-style: italic;
    font-display: swap;
  }
</style>
