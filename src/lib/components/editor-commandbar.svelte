<script>
  import { tick } from 'svelte';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { cn } from '$lib/utils';
  import { fileManager } from '@/runes/fs.svelte.js';
  import { goto, invalidateAll } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte';
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import SaveIcon from '@lucide/svelte/icons/save';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import UndoIcon from '@lucide/svelte/icons/undo-2';
  import RedoIcon from '@lucide/svelte/icons/redo-2';
  import HouseIcon from '@lucide/svelte/icons/house';
  import HelpIcon from '@lucide/svelte/icons/help-circle';
  import ScissorsIcon from '@lucide/svelte/icons/scissors';
  import MaximizeIcon from '@lucide/svelte/icons/maximize';
  import TerminalIcon from '@lucide/svelte/icons/square-terminal';
  import SearchIcon from '@lucide/svelte/icons/search';
  import { getMarkdown } from '@milkdown/kit/utils';
  import { editorViewCtx } from '@milkdown/kit/core';
  import { invoke } from '@tauri-apps/api/core';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { isMod, MOD_KEY } from '$lib/keyboard.svelte.js';

  import * as Tooltip from "$lib/components/ui/tooltip/index.js";

  /**
   * @typedef {Object} Props
   * @property {string} [fileName]
   * @property {number} [currentVersion]
   */

  /** @type {Props} */
  let { fileName, currentVersion = 0 } = $props();

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
   * @property {string} [key] - The actual key to listen for with Meta/Ctrl (e.g., 's').
   * @property {import("svelte").Component} [icon] - Icon component.
   */

  /** @type {Command[]} */
  const commands = [
    {
      cmd: ':c',
      description: 'Create Document Context',
      action: 'createDocumentContext',
      shortcutLabel: '', // No global shortcut to avoid conflict with Cmd+C
      key: ''
    },
    {
      cmd: ':u',
      description: 'Undo (Back one version)',
      action: 'undo',
      shortcutLabel: `${MOD_KEY}Z`,
      key: 'z',
      icon: UndoIcon,
    },
    {
      cmd: ':redo',
      description: 'Redo (Forward to latest branch)',
      action: 'redo',
      shortcutLabel: `${MOD_KEY}⇧Z`,
      key: 'Y',
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
      shortcutLabel: `${MOD_KEY}H`,
      key: 'h',
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
      shortcutLabel: `${MOD_KEY}Q`,
      key: 'q'
    },
    {
      cmd: ':b',
      description: 'Go Home',
      action: 'sidebar',
      shortcutLabel: `${MOD_KEY}B`,
      key: 'b',
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
      cmd: ':shorten',
      description: 'AI Shorten',
      action: 'aiShorten',
      icon: ScissorsIcon,
    },
    {
      cmd: ':expand',
      description: 'AI Expand',
      action: 'aiExpand',
      icon: MaximizeIcon,
    },
    {
      cmd: ':polish',
      description: 'AI Polish',
      action: 'aiPolish',
      icon: AiSparkleIcon,
    },
  ];

  const quickCommands = commands.filter(c => [ ':redo', ':w', ':ai', ':h'].includes(c.cmd));

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
        (cmd) =>
          !directMatches.includes(cmd) &&
          cmd.description.toLowerCase().includes(searchTerm)
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
  function onSave() {
    if (editorState.editor && fileName) {
      console.log("Saving file:", fileName);
      const markdown = editorState.editor.action(getMarkdown());
      fileManager.createNewFile(fileName, markdown);
      invalidateAll();
    }
  }

  /**
   * @param {string} cmd
   */
  async function executeCommand(cmd) {
    // Match either full command (like ":w") or the part without colon (like "w")
    const command = commands.find((c) => c.cmd === cmd || c.cmd.slice(1) === cmd);
    if (!command) return;

    switch (command.action) {
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
        console.debug('[DEBUG aichat] isChatOpen before=%s -> requesting=%s', appState.ui.isChatOpen, !appState.ui.isChatOpen);
        appState.toggleAiChat(!appState.ui.isChatOpen);
        break;
      case 'save':
        onSave();
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
          await invoke('undo_file', { name: fileName });
          await invalidateAll();
          appState.incrementEditorVersion();
        } catch (e) {
          console.error("Undo failed:", e);
        }
        break;
      case 'redo':
        try {
          await invoke('redo_file', { name: fileName });
          await invalidateAll();
          appState.incrementEditorVersion();
        } catch (e) {
          console.error("Redo failed:", e);
        }
        break;
      case 'help':
        appState.toggleHelpModal(!appState.ui.isHelpModalOpen);
        break;
      case 'aiShorten':
      case 'aiExpand':
      case 'aiPolish': {
        const actionMap = {
          'aiShorten': 'LengthShorten',
          'aiExpand': 'LengthExpand',
          'aiPolish': 'CorrectGrammar'
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
    // Optional: Return focus to editor
    if (editorState.editMode) {
      editorState.editor?.action((ctx) => {
        const view = ctx.get(editorViewCtx);
        view.focus();
      });
    }
  }

  /**
   * KEYDOWN HANDLER (Global Capture)
   */
  $effect(() => {
    /** @param {KeyboardEvent} e */
    const handleCaptureKeyDown = (e) => {
      if (e.key === 'Tab' && editorState.editMode) {
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
        if (editorState.editMode) {
          editorState.setEditMode(false);
          editorState?.editor?.action((ctx) => {
            const view = ctx.get(editorViewCtx);
            view.dom.blur();
          });
          e.preventDefault();
          return;
        }
      }

      // 2. Trigger Command Mode with ':'
      if (
        !editorState.editMode &&
        e.key === ':' &&
        !isCommandMode &&
        !appState.ui.isChatOpen &&
        !['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName)
      ) {
        e.preventDefault();
        isCommandMode = true;
        input = ':';
        return;
      }

      // 3. Trigger Edit Mode with Vim-like keys
      // 'i' - Insert at cursor
      if (
        e.key === 'i' &&
        !editorState.editMode &&
        !isCommandMode &&
        !appState.ui.isChatOpen &&
        !['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName)
      ) {
        e.preventDefault();
        editorState.setEditMode(true);
        tick().then(() => {
          const editor = editorState?.editor;
          if (editor) {
            editor.action((ctx) => {
              const view = ctx.get(editorViewCtx);
              view.focus();
            });
          }
        });
        return;
      }

      // 4. Handle Shortcuts
      if (isMod(e) && isCommandMode) {
        console.debug('[DEBUG shortcut blocked] key=%s isCommandMode=%s', e.key, isCommandMode);
      }
      if (isMod(e) && !isCommandMode) {
        // Special check for Z (Undo) and Shift+Z / Y (Redo)
        if (e.key.toLowerCase() === 'z') {
           e.preventDefault();
           e.stopPropagation();
           if (e.shiftKey) {
             executeCommand(':redo');
           } else {
             executeCommand(':u');
           }
           return;
        }
        if (e.key.toLowerCase() === 'y') {
           e.preventDefault();
           e.stopPropagation();
           executeCommand(':redo');
           return;
        }

        // Handle other mapped keys
        const match = commands.find(c => c.key === e.key.toLowerCase());
        console.debug('[DEBUG shortcut match] key=%s matched=%s', e.key, match?.cmd);
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
  <div class="pointer-events-none absolute inset-x-0 top-0 h-px bg-linear-to-r from-transparent via-border to-transparent"></div>

  {#if isCommandMode && suggestions.length > 0}
    <div
      class="absolute bottom-full left-0 right-0 mb-1 mx-4 max-w-2xl md:mx-auto bg-popover border border-border rounded-lg shadow-xl overflow-hidden z-50"
      role="listbox"
    >
      <div class="max-h-64 overflow-y-auto p-1">
        {#each suggestions as suggestion, index (suggestion.cmd)}
          <button
            onclick={() => executeCommand(suggestion.cmd)}
            onfocus={handleFocus}
            class={cn(
              "w-full px-3 py-2 text-left text-sm font-mono flex items-center justify-between rounded-sm transition-colors",
              index === selectedIndex ? "bg-accent text-accent-foreground" : "text-muted-foreground hover:bg-muted"
            )}
            role="option"
            aria-selected={index === selectedIndex}
          >
            <div class="flex items-center gap-2">
              <span class="font-bold text-foreground">{suggestion.cmd}</span>
              <span class="text-xs opacity-80">{suggestion.description}</span>
            </div>
            {#if suggestion.shortcutLabel}
              <kbd class="hidden sm:inline-block pointer-events-none h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-100">
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
      <span class="font-mono text-sm text-muted-foreground px-2 shrink-0">:</span>
      <input
        bind:this={inputRef}
        type="text"
        bind:value={input}
        onkeydown={handleInputKeyDown}
        onblur={handleBlur}
        onfocus={handleFocus}
        placeholder="Type command..."
        class="w-full bg-transparent font-mono text-sm outline-none text-foreground placeholder:text-muted-foreground/50 h-full"
        aria-label="Command input"
      />
    </div>
  {:else}
    <div class="flex items-stretch command-bar__inner h-9 text-[11px] font-mono">
      <!-- File/version state -->
      <div class="flex items-center gap-2 px-3.5 text-muted-foreground/80 shrink-0">
        <span class={cn(
          "tracking-wide transition-colors",
          appState.ui.isHistoryOpen ? "text-success font-semibold" : ""
        )}>
          {appState.ui.isHistoryOpen ? 'HISTORY' : `v${currentVersion}`}
        </span>
      </div>

      <div class="flex-1"></div>

      <!-- Shortcut icons: hover for label + keys -->
      <div class="hidden md:flex items-center gap-1 px-2 text-muted-foreground/60 shrink-0">
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button {...props} onclick={() => executeCommand(':h')} class="flex items-center justify-center size-7 rounded-md hover:text-foreground hover:bg-foreground/5 transition-colors">
                <HelpIcon class="size-3.5" />
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}>Shortcuts help · {MOD_KEY}H</Tooltip.Content>
        </Tooltip.Root>

        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button {...props} onclick={() => { isCommandMode = true; input = ':'; }} class="flex items-center justify-center size-7 rounded-md hover:text-foreground hover:bg-foreground/5 transition-colors">
                <TerminalIcon class="size-3.5" />
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}>Command palette · :</Tooltip.Content>
        </Tooltip.Root>

        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button {...props} onclick={() => appState.toggleCommandMenu(true)} class="flex items-center justify-center size-7 rounded-md hover:text-foreground hover:bg-foreground/5 transition-colors">
                <SearchIcon class="size-3.5" />
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}>Search · {MOD_KEY}K</Tooltip.Content>
        </Tooltip.Root>

        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <button
                {...props}
                onclick={() => {
                  if (!llmManager.modelsLoaded) {
                    const isDownloaded = llmManager.modelStatuses.some(m => m.downloaded);
                    if (isDownloaded) {
                      llmManager.loadModels();
                    } else if (confirm('Download AI Models for local intelligence? (approx 1.5GB)')) {
                      llmManager.loadModels();
                    }
                  } else {
                    appState.toggleAiChat(!appState.ui.isChatOpen);
                  }
                }}
                class="flex items-center justify-center size-7 rounded-md hover:text-foreground hover:bg-foreground/5 transition-colors"
              >
                <AiSparkleIcon class={cn(
                  'size-3.5 transition-colors',
                  (isThinking || llmManager.isLoadModelsInProgress) ? 'animate-pulse' : '',
                  !llmManager.modelsLoaded ? 'text-warning' : ''
                )} />
              </button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="top" portalProps={{}}>
            {#if llmManager.isLoadModelsInProgress}
              Downloading intelligence... {llmManager.loadingProgress}%
            {:else if llmManager.downloadingModelId}
              Model downloading in background
            {:else if !llmManager.modelsLoaded}
              Click to download local AI model
            {:else}
              AI chat · {MOD_KEY}L
            {/if}
          </Tooltip.Content>
        </Tooltip.Root>
      </div>

      <!-- Mobile overflow menu -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <Button {...props} variant="ghost" size="icon" class="h-7 w-7 rounded-none md:hidden shrink-0">
              <HouseIcon class="size-3.5" />
            </Button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content class="{appState.ui.theme} w-56 mr-2" align="end" portalProps={{}}>
          <DropdownMenu.Group>
            {#each quickCommands as qCmd (qCmd.cmd)}
              <DropdownMenu.Item onclick={() => executeCommand(qCmd.cmd)} class="" inset={false}>
                <div class="flex items-center gap-2 flex-1">
                  <qCmd.icon class="size-3.5" />
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