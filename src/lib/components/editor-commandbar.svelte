<script>
  import { tick } from 'svelte';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { appState, DENSITY_FONT_SIZE } from '$lib/runes/app.svelte.js';
  import { cn } from '$lib/utils';
  import { goto, invalidateAll } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { configManager } from '@/runes/config.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte';
  import { gitManager } from '@/runes/git.svelte.js';
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import SaveIcon from '@lucide/svelte/icons/save';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import UndoIcon from '@lucide/svelte/icons/undo-2';
  import RedoIcon from '@lucide/svelte/icons/redo-2';
  import HouseIcon from '@lucide/svelte/icons/house';
  import HelpIcon from '@lucide/svelte/icons/help-circle';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { startNewWriting } from '$lib/new-writing.js';
  import { dirOf } from '$lib/runes/fs.svelte.js';
  import TerminalIcon from '@lucide/svelte/icons/square-terminal';
  import SearchIcon from '@lucide/svelte/icons/search';
  import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
  import { getMarkdown } from '@milkdown/kit/utils';
  import { editorViewCtx } from '@milkdown/kit/core';
  import { invoke } from '@tauri-apps/api/core';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { isMod, MOD_KEY } from '$lib/keyboard.svelte.js';
  import { toast } from '$lib/toast.js';

  import * as Tooltip from '$lib/components/ui/tooltip/index.js';

  let isDownloadDialogOpen = $state(false);

  /**
   * @typedef {Object} Props
   * @property {string} [fileName]
   * @property {number} [currentVersion]
   */

  /** @type {Props & { isDraft?: boolean }} */
  let { fileName, currentVersion = 0, isDraft = false } = $props();

  // Runes & State
  let isThinking = $derived(llmManager.isLoading);
  let isCommandMode = $state(false);
  let input = $state('');
  let selectedIndex = $state(0);
  let inputRef = $state(/** @type {HTMLInputElement | null} */ (null));
  let blurTimeout = $state(/** @type {ReturnType<typeof setTimeout> | null} */ (null));

  const BLUR_DELAY = 200;

  /**
   * @typedef {Object} Command
   * @property {string} cmd - The command string (e.g., ":w").
   * @property {string} description - A brief description.
   * @property {string} action - Action identifier.
   * @property {string} [shortcutLabel] - Visual shortcut display (e.g. '⌘S').
   * @property {string} [key] - Key to listen for together with Meta/Ctrl (e.g., 's'). Empty = no shortcut.
   * @property {boolean} [shift] - Shortcut also needs Shift.
   * @property {boolean} [alt] - Shortcut also needs Alt/Option.
   * @property {import("svelte").Component} [icon] - Icon component.
   */

  /**
   * Shortcuts deliberately avoid keys the editor or the OS already own:
   * ⌘B/⌘I/⌘E (marks), ⌘Z/⌘⇧Z (editor undo/redo), ⌘Q (quit), ⌘H (hide), ⌘W (close window).
   */

  let aiEnabled = $derived(configManager.config?.ai_enabled ?? false);
  let defaultFontSize = $derived(DENSITY_FONT_SIZE[appState.ui.density]);
  let zoomPercent = $derived(Math.round((appState.ui.fontSize / defaultFontSize) * 100));

  /** @type {Command[]} */
  const allCommands = [
    {
      cmd: ':c',
      description: 'Create Writing Context',
      action: 'createDocumentContext',
      shortcutLabel: '', // No global shortcut to avoid conflict with Cmd+C
      key: '',
    },
    {
      cmd: ':n',
      description: 'New writing (same folder)',
      action: 'new',
      shortcutLabel: `${MOD_KEY}N`,
      key: '', // handled globally in the root layout
      icon: PlusIcon,
    },
    {
      cmd: ':u',
      description: 'Previous version',
      action: 'undo',
      shortcutLabel: `${MOD_KEY}⌥Z`,
      key: 'z',
      alt: true,
      icon: UndoIcon,
    },
    {
      cmd: ':redo',
      description: 'Next version (latest branch)',
      action: 'redo',
      shortcutLabel: `${MOD_KEY}⌥⇧Z`,
      key: 'z',
      alt: true,
      shift: true,
      icon: RedoIcon,
    },
    {
      cmd: ':history',
      description: 'Toggle history sidebar',
      action: 'toggleHistory',
      shortcutLabel: `${MOD_KEY}U`,
      key: 'u',
      icon: HistoryIcon,
    },
    {
      cmd: ':h',
      description: 'Show shortcuts help',
      action: 'help',
      shortcutLabel: `${MOD_KEY}/`,
      key: '/',
      icon: HelpIcon,
    },
    {
      cmd: ':w',
      description: 'Save file',
      action: 'save',
      shortcutLabel: `${MOD_KEY}S`,
      key: 's',
      icon: SaveIcon,
    },
    {
      cmd: ':q',
      description: 'Close file',
      action: 'close',
      shortcutLabel: '',
      key: '',
    },
    {
      cmd: ':b',
      description: 'Go Home',
      action: 'sidebar',
      shortcutLabel: `${MOD_KEY}⇧H`,
      key: 'h',
      shift: true,
      icon: HouseIcon,
    },
    {
      cmd: ':ai',
      description: 'Toggle AI Chat',
      action: 'aichat',
      shortcutLabel: `${MOD_KEY}L`,
      key: 'l',
      icon: AiSparkleIcon,
    },
    {
      cmd: ':push',
      description: 'Sync writing to GitHub (commit & push)',
      action: 'push',
      shortcutLabel: '',
      key: '',
      icon: UploadCloudIcon,
    },
  ];

  let commands = $derived(
    aiEnabled ? allCommands : allCommands.filter((c) => !['ai', 'c'].includes(c.cmd.slice(1)))
  );

  let quickCommands = $derived(
    commands.filter((c) => [':redo', ':w', ':ai', ':h'].includes(c.cmd))
  );

  let suggestions = $derived.by(() => {
    if (input === ':') return commands;

    if (input.length >= 1) {
      const term = input.toLowerCase();
      const searchTerm = term.startsWith(':') ? term.slice(1) : term;

      // Tier 1: Direct matches (command string starts with term or searchTerm)
      const directMatches = commands.filter(
        (cmd) =>
          cmd.cmd.toLowerCase().startsWith(term) ||
          cmd.cmd.toLowerCase().slice(1).startsWith(searchTerm)
      );

      // Tier 2: Description matches (description contains searchTerm)
      const descriptionMatches = commands.filter(
        (cmd) => !directMatches.includes(cmd) && cmd.description.toLowerCase().includes(searchTerm)
      );

      return [...directMatches, ...descriptionMatches];
    }
    return [];
  });

  $effect(() => {
    suggestions;
    selectedIndex = 0;
  });

  $effect(() => {
    if (isCommandMode && inputRef) {
      tick().then(() => inputRef?.focus());
    }
  });

  /**
   * Actions
   */
  /** Same path as autosave, so drafts get named and created the same way. */
  async function onSave() {
    if (editorState.flushSave) await editorState.flushSave();
  }

  /**
   * @param {string} cmd
   */
  async function executeCommand(cmd) {
    // Match either full command (like ":w") or the part without colon (like "w")
    const command = commands.find((c) => c.cmd === cmd || c.cmd.slice(1) === cmd);
    if (!command) return;

    // Drafts have no versions yet and nothing to push.
    if (isDraft && ['undo', 'redo', 'toggleHistory', 'push'].includes(command.action)) {
      toast.info('Start writing first', 'Versions appear after the first save.');
      closeCommandMode();
      return;
    }

    switch (command.action) {
      case 'new':
        await onSave();
        startNewWriting(dirOf(fileName));
        break;
      case 'createDocumentContext':
        memoryManager.createDocumentContext();
        break;
      case 'toggleHistory':
        appState.toggleChatHistory();
        break;
      case 'sidebar':
        goto(resolve('/'));
        break;
      case 'aichat':
        appState.toggleAiChat(!appState.ui.isChatOpen);
        break;
      case 'save':
        onSave();
        break;
      case 'push':
        if (!gitManager.user) {
          toast.info('Not connected to GitHub', 'Log in under Settings → Cloud Sync first.');
          break;
        }
        await onSave();
        await gitManager.commitAndPush(`Update writing — ${new Date().toLocaleString()}`);
        if (gitManager.pushResult === 'error') {
          toast.error('Push failed', gitManager.error);
        } else {
          toast.success('Pushed to GitHub');
        }
        break;
      case 'close':
        goto(resolve('/'));
        break;
      case 'saveAndClose':
        onSave();
        goto(resolve('/'));
        break;
      case 'undo':
        try {
          const moved = await invoke('undo_file', { name: fileName });
          if (moved === null || moved === undefined) {
            toast.info('Already at the oldest version');
            break;
          }
          await invalidateAll();
          appState.incrementEditorVersion();
        } catch (e) {
          console.error('Undo failed:', e);
          toast.error('Could not go to previous version', e);
        }
        break;
      case 'redo':
        try {
          const moved = await invoke('redo_file', { name: fileName });
          if (moved === null || moved === undefined) {
            toast.info('Already at the latest version');
            break;
          }
          await invalidateAll();
          appState.incrementEditorVersion();
        } catch (e) {
          console.error('Redo failed:', e);
          toast.error('Could not go to next version', e);
        }
        break;
      case 'help':
        appState.toggleHelpModal(!appState.ui.isHelpModalOpen);
        break;
      case 'aiShorten':
      case 'aiExpand':
      case 'aiPolish': {
        const actionMap = {
          aiShorten: 'LengthShorten',
          aiExpand: 'LengthExpand',
          aiPolish: 'CorrectGrammar',
        };
        const milkdownAction = actionMap[command.action];

        // Get text from selection if possible, otherwise use full document
        let textToEdit = '';
        if (editorState.editor) {
          editorState.editor.action((ctx) => {
            const view = ctx.get(editorViewCtx);
            const { state } = view;
            const { from, to } = state.selection;
            if (from !== to) {
              textToEdit = state.doc.textBetween(from, to, ' ');
            } else {
              // Fallback to full doc if no selection?
              // Actually, GrammarBox works per-paragraph.
              // For now, let's just use selection if available.
              textToEdit = getMarkdown()(ctx);
            }
          });
        }

        if (textToEdit) {
          llmManager.sendEditActionMessage(textToEdit, milkdownAction);
        }
        break;
      }
    }

    closeCommandMode();
  }

  function closeCommandMode() {
    isCommandMode = false;
    input = '';
    // Return focus to editor
    editorState.editor?.action((ctx) => {
      const view = ctx.get(editorViewCtx);
      view.focus();
    });
  }

  /**
   * KEYDOWN HANDLER (Global Capture)
   */
  $effect(() => {
    /** @param {KeyboardEvent} e */
    const handleCaptureKeyDown = (e) => {
      const target = e.target instanceof HTMLElement ? e.target : null;
      const inEditor = !!target?.closest('.ProseMirror');

      // Tab only means "indent" while the caret is in the document. Anywhere
      // else (command input, chat box, dialogs) it keeps its normal meaning.
      if (e.key === 'Tab' && inEditor) {
        const editor = editorState?.editor;
        if (editor) {
          let inList = false;
          editor.action((ctx) => {
            const view = ctx.get(editorViewCtx);
            const { state } = view;
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
            const view = ctx.get(editorViewCtx);
            const { state, dispatch } = view;
            dispatch(state.tr.insertText('  '));
          });
        } else {
          e.preventDefault();
        }
        return;
      }

      if (e.key === 'Escape') {
        if (isCommandMode) {
          e.preventDefault();
          e.stopPropagation();
          closeCommandMode();
          return;
        }
        if (appState.ui.isChatOpen) {
          appState.toggleAiChat(false);
          e.preventDefault();
          return;
        }
      }

      // 2. Handle Shortcuts
      if (isMod(e) && !isCommandMode) {
        // Trigger Command Mode with Cmd/Ctrl+Shift+P (standard command palette shortcut)
        if (e.shiftKey && !e.altKey && e.key.toLowerCase() === 'p') {
          e.preventDefault();
          e.stopPropagation();
          isCommandMode = true;
          input = ':';
          return;
        }

        // Exact modifier match, so ⌘Z (editor undo) and ⌘⌥Z (version undo) stay distinct.
        const key = e.key.toLowerCase();
        const match = commands.find(
          (c) =>
            c.key &&
            c.key === key &&
            !!c.shift === e.shiftKey &&
            !!c.alt === e.altKey
        );
        if (match) {
          e.preventDefault();
          e.stopPropagation();
          executeCommand(match.cmd);
        }
      }
    };

    window.addEventListener('keydown', handleCaptureKeyDown, { capture: true });
    return () => {
      window.removeEventListener('keydown', handleCaptureKeyDown, { capture: true });
    };
  });

  // Input Handling
  /** @param {KeyboardEvent} e */
  function handleInputKeyDown(e) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % suggestions.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + suggestions.length) % suggestions.length;
    } else if (e.key === 'Tab') {
      e.preventDefault();
      if (suggestions[selectedIndex]) {
        input = suggestions[selectedIndex].cmd;
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (suggestions[selectedIndex]) {
        executeCommand(suggestions[selectedIndex].cmd);
      } else {
        executeCommand(input);
      }
    }
  }

  function handleBlur() {
    blurTimeout = setTimeout(() => {
      closeCommandMode();
    }, BLUR_DELAY);
  }

  function handleFocus() {
    if (blurTimeout) {
      clearTimeout(blurTimeout);
      blurTimeout = null;
    }
  }
</script>

<div class="relative bg-background z-40 command-bar">
  <div
    class="pointer-events-none absolute inset-x-0 top-0 h-px bg-linear-to-r from-transparent via-border to-transparent"
  ></div>

  {#if isCommandMode && suggestions.length > 0}
    <div
      class="absolute bottom-full left-0 right-0 mb-1 mx-4 max-w-2xl md:mx-auto bg-popover border border-border rounded-xl shadow-lg overflow-hidden z-50"
      role="listbox"
    >
      <div class="max-h-64 overflow-y-auto p-1">
        {#each suggestions as suggestion, index (suggestion.cmd)}
          <button
            onclick={() => executeCommand(suggestion.cmd)}
            onfocus={handleFocus}
            class={cn(
              'w-full px-3 py-2 text-left text-sm font-mono flex items-center justify-between rounded-sm transition-colors',
              index === selectedIndex
                ? 'bg-accent text-accent-foreground'
                : 'text-muted-foreground hover:bg-muted'
            )}
            role="option"
            aria-selected={index === selectedIndex}
          >
            <div class="flex items-center gap-2">
              <span class="font-bold text-foreground">{suggestion.cmd}</span>
              <span class="text-xs opacity-80">{suggestion.description}</span>
            </div>
            {#if suggestion.shortcutLabel}
              <kbd
                class="hidden sm:inline-block pointer-events-none h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[0.625rem] font-medium text-muted-foreground opacity-100"
              >
                {suggestion.shortcutLabel}
              </kbd>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if isCommandMode}
    <!-- Command-line mode: takes over the whole statusline, like vim's ":" prompt -->
    <div class="flex items-center command-bar__inner h-8">
      <input
        bind:this={inputRef}
        type="text"
        bind:value={input}
        onkeydown={handleInputKeyDown}
        onblur={handleBlur}
        onfocus={handleFocus}
        placeholder="Type command..."
        class="w-full bg-transparent font-mono text-sm outline-none text-foreground placeholder:text-muted-foreground/50 h-full px-2"
        aria-label="Command input"
 />
    </div>
  {:else}
    <div class="flex items-stretch command-bar__inner h-9 text-[0.6875rem] font-mono">
      <!-- File/version state -->
      <div class="flex items-center gap-2 px-3.5 text-muted-foreground/80 shrink-0">
        <span class="tracking-wide">
          {isDraft ? 'DRAFT' : appState.ui.isHistoryOpen ? 'HISTORY' : `v${currentVersion}`}
        </span>
        {#if zoomPercent !== 100}
          <span class="tracking-wide">{zoomPercent}%</span>
        {/if}
      </div>

      <div class="flex-1"></div>

      <!-- Shortcut icons: hover for label + keys -->
      <div class="hidden md:flex items-center gap-1 px-2 text-muted-foreground/60 shrink-0">
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button
                {...props}
                onclick={() => executeCommand(':h')}
                class="flex items-center justify-center size-7 rounded-md hover:text-foreground hover:bg-foreground/5 transition-colors"
              >
                <HelpIcon strokeWidth={1.5} class="size-3.5" />
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}>Shortcuts help · {MOD_KEY}H</Tooltip.Content>
        </Tooltip.Root>

        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button
                {...props}
                onclick={() => {
                  isCommandMode = true;
                  input = ':';
                }}
                class="flex items-center justify-center size-7 rounded-md hover:text-foreground hover:bg-foreground/5 transition-colors"
              >
                <TerminalIcon strokeWidth={1.5} class="size-3.5" />
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}
            >Command palette · {MOD_KEY}⇧P</Tooltip.Content
          >
        </Tooltip.Root>

        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button
                {...props}
                onclick={() => appState.toggleCommandMenu(true)}
                class="flex items-center justify-center size-7 rounded-md hover:text-foreground hover:bg-foreground/5 transition-colors"
              >
                <SearchIcon strokeWidth={1.5} class="size-3.5" />
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}>Search · {MOD_KEY}K</Tooltip.Content>
        </Tooltip.Root>

        {#if aiEnabled}
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button
                {...props}
                onclick={() => {
                  if (llmManager.isLoadModelsInProgress || llmManager.downloadingModelId) return;
                  if (!llmManager.modelsLoaded) {
                    const isDownloaded = llmManager.modelStatuses.some((m) => m.downloaded);
                    if (isDownloaded) {
                      llmManager.loadModels();
                    } else {
                      isDownloadDialogOpen = true;
                    }
                  } else {
                    appState.toggleAiChat(!appState.ui.isChatOpen);
                  }
                }}
                class="flex items-center justify-center size-7 rounded-md hover:text-foreground hover:bg-foreground/5 transition-colors"
              >
                <AiSparkleIcon strokeWidth={1.5}
                  class={cn(
                    'size-3.5 transition-colors',
                    isThinking || llmManager.isLoadModelsInProgress ? 'animate-pulse' : '',
                    !llmManager.modelsLoaded ? 'text-warning' : ''
                  )}
 />
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}>
            {#if llmManager.isLoadModelsInProgress}
              Loading AI model… {llmManager.loadingProgress}%
            {:else if llmManager.downloadingModelId}
              Downloading model… {llmManager.loadingProgress}%
            {:else if !llmManager.modelsLoaded}
              Click to set up local AI
            {:else}
              AI chat · {MOD_KEY}L
            {/if}
          </Tooltip.Content>
        </Tooltip.Root>
        {/if}
      </div>

      <Dialog.Root bind:open={isDownloadDialogOpen}>
        <Dialog.Content class="sm:max-w-[420px] font-writer" portalProps={{}}>
          <Dialog.Header class="">
            <Dialog.Title class="text-xl font-normal">Set up local AI</Dialog.Title>
            <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
              Memoire runs AI entirely on your machine. The first time it needs to download a
              language model (about 1.5 GB). Nothing you write leaves this device.
            </Dialog.Description>
          </Dialog.Header>
          <Dialog.Footer class="mt-6 flex gap-2">
            <Button variant="ghost" class="flex-1" onclick={() => (isDownloadDialogOpen = false)}>
              Not now
            </Button>
            <Button
              class="flex-1"
              onclick={() => {
                isDownloadDialogOpen = false;
                llmManager.loadModels();
              }}
            >
              Download
            </Button>
          </Dialog.Footer>
        </Dialog.Content>
      </Dialog.Root>

      <!-- Mobile overflow menu -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <Button
              {...props}
              variant="ghost"
              size="icon"
              class="h-7 w-7 rounded-none md:hidden shrink-0"
            >
              <HouseIcon strokeWidth={1.5} class="size-3.5" />
            </Button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content class="{appState.ui.theme} w-56 mr-2" align="end" portalProps={{}}>
          <DropdownMenu.Group>
            {#each quickCommands as qCmd (qCmd.cmd)}
              <DropdownMenu.Item onclick={() => executeCommand(qCmd.cmd)} class="" inset={false}>
                <div class="flex items-center gap-2 flex-1">
                  <qCmd.icon strokeWidth={1.5} class="size-3.5" />
                  <span class="font-mono text-sm">{qCmd.description}</span>
                </div>
                <DropdownMenu.Shortcut class="">{qCmd.shortcutLabel}</DropdownMenu.Shortcut>
              </DropdownMenu.Item>
            {/each}
          </DropdownMenu.Group>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </div>
  {/if}
</div>
