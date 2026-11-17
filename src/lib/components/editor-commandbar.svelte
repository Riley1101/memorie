<script>
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { appState, DENSITY_FONT_SIZE } from '$lib/runes/app.svelte.js';
  import { llmManager } from '$lib/runes/llm.svelte.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import EditorStats from '$lib/components/editor-stats.svelte';
  import { editorViewCtx } from '@milkdown/kit/core';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { isMod, shortcut } from '$lib/keyboard.svelte.js';
  import {
    editorSession,
    aiSetup,
    editorCommands,
    runEditorCommand,
  } from '$lib/editor-commands.svelte.js';

  /**
   * The quiet dock floating over the foot of the page: the count and goal,
   * and one way into every command (the palette, ⌘K). It also owns the
   * editor's keyboard shortcuts, since it exists exactly while a writing is open.
   */

  /** @type {{ fileName?: string, isDraft?: boolean }} */
  let { fileName, isDraft = false } = $props();

  // Tell the palette which writing is open, and that none is once we leave.
  $effect(() => {
    editorSession.fileName = fileName ?? null;
    editorSession.isDraft = isDraft;
    return () => {
      editorSession.fileName = null;
      editorSession.isDraft = false;
    };
  });

  let defaultFontSize = $derived(DENSITY_FONT_SIZE[appState.ui.density]);
  let zoomPercent = $derived(Math.round((appState.ui.fontSize / defaultFontSize) * 100));

  $effect(() => {
    /** @param {KeyboardEvent} e */
    const handleCaptureKeyDown = (e) => {
      const target = e.target instanceof HTMLElement ? e.target : null;
      const inEditor = !!target?.closest('.ProseMirror');

      // Tab only means "indent" while the caret is in the document. Anywhere
      // else (the palette, chat box, dialogs) it keeps its normal meaning.
      if (e.key === 'Tab' && inEditor) {
        const editor = editorState?.editor;
        if (editor) {
          let inList = false;
          editor.action((ctx) => {
            const { state } = ctx.get(editorViewCtx);
            const fromPos = state.selection['$from'];
            for (let d = fromPos.depth; d > 0; d--) {
              if (fromPos.node(d).type.name.includes('list')) {
                inList = true;
                break;
              }
            }
          });

          if (inList) return;

          e.preventDefault();
          editor.action((ctx) => {
            const { state, dispatch } = ctx.get(editorViewCtx);
            dispatch(state.tr.insertText('  '));
          });
        } else {
          e.preventDefault();
        }
        return;
      }

      if (e.key === 'Escape') {
        if (appState.ui.isChatOpen) {
          appState.toggleAiChat(false);
          e.preventDefault();
          return;
        }
        // Only when nothing else (a dialog, a popover) is going to take the Escape.
        if (writingState.focusMode && !document.querySelector('[role="dialog"], [data-slot="popover-content"]')) {
          e.preventDefault();
          writingState.toggleFocusMode(false);
          return;
        }
      }

      if (!isMod(e)) return;

      // Shortcuts belong to the document. In a text field (the chat box, the
      // title, a dialog's input) the writer is typing, not driving panels —
      // only Save is still meant to work there.
      const inTextField =
        !inEditor &&
        !!target &&
        (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);

      // Exact modifier match, so ⌘Z (editor undo) and ⌘⌥Z (version undo) stay distinct.
      // Letters also match by physical key: on a Mac, Option turns M into µ and Z into Ω.
      const key = e.key.toLowerCase();
      const match = editorCommands().find(
        (c) =>
          c.key &&
          (c.key === key || (/^[a-z]$/.test(c.key) && e.code === `Key${c.key.toUpperCase()}`)) &&
          !!c.shift === e.shiftKey &&
          !!c.alt === e.altKey
      );
      if (!match) return;
      if (inTextField && match.action !== 'save') return;
      e.preventDefault();
      e.stopPropagation();
      runEditorCommand(match);
    };

    window.addEventListener('keydown', handleCaptureKeyDown, { capture: true });
    return () => {
      window.removeEventListener('keydown', handleCaptureKeyDown, { capture: true });
    };
  });
</script>

<!-- Floats over the last lines of the page, so the text shows through the glass. -->
<div class="absolute inset-x-0 bottom-3 z-40 flex justify-center px-4 pointer-events-none">
  <div
    class="dock-glass pointer-events-auto flex items-center gap-2 h-10 max-w-full px-1 rounded-full text-meta tabular-nums text-muted-foreground"
  >
    {#if fileName}
      <EditorStats {fileName} {isDraft} />
    {/if}
    {#if zoomPercent !== 100}
      <span class="shrink-0 px-1">{zoomPercent}%</span>
    {/if}
    <button
      type="button"
      onclick={() => appState.toggleCommandMenu(true)}
      aria-label="Commands"
      title="Commands"
      class="flex items-center h-8 px-3 shrink-0 rounded-full bg-foreground/6 text-foreground hover:bg-foreground/10 transition-colors"
    >
      <kbd class="font-sans">{shortcut('K')}</kbd>
    </button>
  </div>
</div>

<Dialog.Root bind:open={aiSetup.open}>
  <Dialog.Content class="sm:max-w-[420px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal font-writer">Set up local AI</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground pt-2">
        Memoire runs AI entirely on your machine. The first time it needs to download a
        language model (about 1.5 GB). Nothing you write leaves this device.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-6 flex gap-2">
      <Button variant="ghost" class="flex-1" onclick={() => (aiSetup.open = false)}>
        Not now
      </Button>
      <Button
        class="flex-1"
        onclick={() => {
          aiSetup.open = false;
          llmManager.loadModels();
        }}
      >
        Download model
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<style>
  /* Liquid glass: a translucent, blurred and saturated pane with a bright
     rim on top, a soft inner shade below and a light drop shadow. */
  .dock-glass {
    background: color-mix(in oklch, var(--popover) 55%, transparent);
    backdrop-filter: blur(18px) saturate(180%);
    -webkit-backdrop-filter: blur(18px) saturate(180%);
    border: 1px solid color-mix(in oklch, var(--foreground) 8%, transparent);
    box-shadow:
      inset 0 1px 0 0 color-mix(in oklch, white 70%, transparent),
      inset 0 -1px 0 0 color-mix(in oklch, var(--foreground) 4%, transparent),
      0 1px 2px color-mix(in oklch, black 6%, transparent),
      0 8px 24px -6px color-mix(in oklch, black 14%, transparent);
  }

  :global(.dark) .dock-glass {
    background: color-mix(in oklch, var(--popover) 50%, transparent);
    border-color: color-mix(in oklch, white 10%, transparent);
    box-shadow:
      inset 0 1px 0 0 color-mix(in oklch, white 14%, transparent),
      inset 0 -1px 0 0 color-mix(in oklch, black 20%, transparent),
      0 8px 28px -6px color-mix(in oklch, black 50%, transparent);
  }

  @media (prefers-reduced-transparency: reduce) {
    .dock-glass {
      background: var(--popover);
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
    }
  }
</style>
