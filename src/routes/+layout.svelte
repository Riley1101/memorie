<script>
  import '../app.css';
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import SidebarLeft from '$lib/components/sidebar-left.svelte';

  import { appState } from '@/runes/app.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { onDestroy, onMount } from 'svelte';

  /**
   * Initializes the file manager and retrieves the list of files.
   */
  fileManager.getRecents();

  let { children } = $props();

  onMount(() => {
    llmManager.setupListeners();
  });

  onDestroy(() => {
    llmManager.destroy();
  });
</script>

<div class="dark font-writer font-normal w-full h-screen">

  <Sidebar.Provider
    bind:open={() => appState.ui.isSidebarOpen, (newOpen) => {
    appState.toggleSidebar(newOpen)
}}>
    <SidebarLeft />
    <Sidebar.Inset>
      <div class="flex flex-1 flex-col gap-4 text-foreground overflow-hidden relative">
        {@render children()}
      </div>
    </Sidebar.Inset>
  </Sidebar.Provider>
</div>

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
