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
  import MenuIcon from '@lucide/svelte/icons/menu';
  import SaveIcon from '@lucide/svelte/icons/save';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import UndoIcon from '@lucide/svelte/icons/undo-2';
  import RedoIcon from '@lucide/svelte/icons/redo-2';
  import SidebarIcon from '@lucide/svelte/icons/panel-left';
  import HelpIcon from '@lucide/svelte/icons/help-circle';
  import { getMarkdown } from '@milkdown/kit/utils';
  import { editorViewCtx } from '@milkdown/kit/core';
  import { TextSelection } from '@milkdown/kit/prose/state';
  import { invoke } from '@tauri-apps/api/core';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { isMod, MOD_KEY } from '$lib/keyboard.svelte.js';

  /**
   * @typedef {Object} Props
   * @property {string} [fileName]
   * @property {number} [currentVersion]
   */

  /** @type {Props} */
  let { fileName, currentVersion = 0 } = $props();

  // Runes & State
  let isThinking = $derived(llmManager.isLoading);
  let isHistoryVisible = $state(false); // Changed to state if you intend to toggle it
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
      cmd: ':help',
      description: 'Show shortcuts help',
      action: 'help',
      shortcutLabel: '',
      key: ''
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
      cmd: ':wq',
      description: 'Save and close file',
      action: 'saveAndClose',
      shortcutLabel: `${MOD_KEY}W`,
      key: 'w'
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
      description: 'Toggle Sidebar',
      action: 'sidebar',
      shortcutLabel: `${MOD_KEY}B`,
      key: 'b',
      icon: SidebarIcon,
    },
    {
      cmd: ':ai',
      description: 'Toggle AI Chat',
      action: 'aichat',
      shortcutLabel: `${MOD_KEY}L`,
      key: 'l',
      icon: AiSparkleIcon,
    },
  ];

  const quickCommands = commands.filter(c => [ ':redo', ':w', ':b', ':ai', ':h'].includes(c.cmd));

  let suggestions = $derived.by(() => {
    if (input === ':') return commands;

    if (input.length > 1) {
      const term = input.toLowerCase();
      const searchTerm = term.startsWith(':') ? term.slice(1) : term;

      return commands.filter(
        (cmd) =>
          cmd.cmd.toLowerCase().startsWith(term) ||
          cmd.description.toLowerCase().includes(searchTerm)
      );
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
    const command = commands.find((c) => c.cmd === cmd);
    if (!command) return;

    switch (command.action) {
      case 'createDocumentContext':
        memoryManager.createDocumentContext();
        break;
      case 'toggleHistory':
        appState.toggleChatHistory();
        break;
      case 'sidebar':
        appState.toggleSidebar(!appState.ui.isSidebarOpen);
        break;
      case 'aichat':
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
        } catch (e) {
          console.error("Undo failed:", e);
        }
        break;
      case 'redo':
        try {
          await invoke('redo_file', { name: fileName });
          await invalidateAll();
        } catch (e) {
          console.error("Redo failed:", e);
        }
        break;
      case 'help':
        appState.toggleHelpModal(!appState.ui.isHelpModalOpen);
        break;
    }

    closeCommandMode();
  }

  function closeCommandMode() {
    isCommandMode = false;
    input = '';
    // Optional: Return focus to editor
    if (editorState.editMode) {
      editorState.editor?.commands?.focus();
    }
  }

  /**
   * KEYDOWN HANDLER (Global Capture)
   */
  $effect(() => {
    /** @param {KeyboardEvent} e */
    const handleCaptureKeyDown = (e) => {
      // 0. Prevent browser default Tab behavior in insert mode
      if (e.key === 'Tab' && editorState.editMode) {
        e.preventDefault();
        // Optionally insert tab/spaces in editor
        const editor = editorState?.editor;
        if (editor) {
          editor.action((ctx) => {
            const view = ctx.get(editorViewCtx);
            const { state, dispatch } = view;
            // Insert 2 spaces for tab
            dispatch(state.tr.insertText('  '));
          });
        }
        return;
      }
      
      // 1. Handle Escape Priority
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
          editorState?.editor?.commands?.blur();
          e.preventDefault();
          return;
        }
      }

      // 2. Trigger Command Mode with ':'
      if (
        !editorState.editMode &&
        e.key === ':' &&
        !isCommandMode &&
        document.activeElement?.tagName !== 'INPUT'
      ) {
        e.preventDefault();
        isCommandMode = true;
        input = ':';
        return;
      }

      // 3. Trigger Edit Mode with Vim-like keys
      // 'i' - Insert at cursor
      if (e.key === 'i' && !editorState.editMode && !isCommandMode) {
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
        // Allow executing exact match even if not selected
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

<div class="relative bg-background border-t border-border z-40">

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

  <div class="flex items-center px-4 py-2 h-10">

    <div class="flex items-center gap-4 text-xs font-mono text-muted-foreground shrink-0">
      <div class="flex items-center gap-1.5">
        <AiSparkleIcon class={cn('size-3.5', isThinking ? 'animate-pulse text-yellow-400' : '')} />
        <span class={cn("transition-colors", isHistoryVisible ? "text-green-500 font-bold" : "")}>
          {isHistoryVisible ? 'HISTORY' : `v${currentVersion}`}
        </span>
      </div>
    </div>

    <div class="flex-1 flex items-center px-4 overflow-hidden">
      {#if isCommandMode}
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
      {:else}
        <div class="hidden md:flex items-center gap-3 animate-in fade-in slide-in-from-left-2 duration-200">
          <div class="flex items-center gap-2 pr-2 border-r border-muted-foreground/20">
            <span class={cn(
              "px-1.5 py-0.5 rounded text-[10px] font-bold uppercase transition-all tracking-wider",
              editorState.editMode 
                ? "bg-blue-500/20 text-blue-500 border border-blue-500/30" 
                : "bg-muted text-muted-foreground border border-transparent"
            )}>
              {editorState.editMode ? 'INSERT' : 'NORMAL'}
            </span>
          </div>

          {#each quickCommands as qCmd (qCmd.cmd)}
            <button
              onclick={() => executeCommand(qCmd.cmd)}
              class="group flex items-center gap-1.5 px-2 py-1 rounded-md hover:bg-muted/50 transition-all border border-transparent hover:border-border/50"
              title={qCmd.description}
            >
              {#if qCmd.icon}
                <qCmd.icon class="size-3.5 text-muted-foreground group-hover:text-primary transition-colors" />
              {/if}
              {#if qCmd.shortcutLabel}
                <span class="text-[10px] font-mono text-muted-foreground/60 group-hover:text-foreground transition-colors">
                  {qCmd.shortcutLabel}
                </span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="shrink-0">
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <Button {...props} variant="ghost" size="icon" class="h-6 w-6 md:hidden">
              <MenuIcon class="size-3.5" />
            </Button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content class="dark w-56 mr-2" align="end">
          <DropdownMenu.Group>
            {#each quickCommands as qCmd (qCmd.cmd)}
              <DropdownMenu.Item onclick={() => executeCommand(qCmd.cmd)}>
                <DropdownMenu.Icon>
                  <qCmd.icon />
                </DropdownMenu.Icon>
                <span class="font-mono text-sm flex-1">{qCmd.description}</span>
                <DropdownMenu.Shortcut>{qCmd.shortcutLabel}</DropdownMenu.Shortcut>
              </DropdownMenu.Item>
            {/each}
          </DropdownMenu.Group>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      {#if !isCommandMode}
        <div class="hidden md:flex items-center gap-2 text-[10px] font-mono text-muted-foreground opacity-70">
          <span class="flex items-center gap-1">
            <kbd class="pointer-events-none inline-flex h-4 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium">:</kbd>
            <span>cmd</span>
          </span>
          <span class="flex items-center gap-1 ml-2">
            <kbd class="pointer-events-none inline-flex h-4 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium">{MOD_KEY}K</kbd>
            <span>search</span>
          </span>
        </div>
      {/if}
    </div>
  </div>
</div>