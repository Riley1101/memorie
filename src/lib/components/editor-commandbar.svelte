<script>
  import { tick } from 'svelte';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { cn } from '$lib/utils';
  import { fileManager } from '@/runes/fs.svelte.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { invalidateAll } from '$app/navigation';
  import { llmManager } from '@/runes/llm.svelte.js';
  import { memoryManager } from '@/runes/memory.svelte';
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import MenuIcon from '@lucide/svelte/icons/menu';
  import { getMarkdown } from '@milkdown/kit/utils';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Button } from '$lib/components/ui/button/index.js';

  /**
   * @type {{fileName?:string ,body?: string , currentVersion?: number}}
   */
  let data = $props();

  let fileName = $derived(data.fileName);
  let isThinking = $derived(llmManager.isLoading);

  const BLUR_DELAY = 200;

  /**
   * @description Handles the save action for the editor content.
   */
  function onSave() {
    if (editorState.editor) {
      const markdown = editorState.editor.action(getMarkdown());
      if (fileName) {
        fileManager.createNewFile(fileName, markdown);
        invalidateAll();
      }
    }
  }

  /**
   * @typedef {Object} Command
   * @property {string} cmd - The command string (e.g., ":w").
   * @property {string} description - A brief description of the command.
   * @property {string} action - An identifier for the action to take.
   * @property {string} [shortcut] - Optional keyboard shortcut display.
   */

  /**
   * @type {boolean}
   */
  let isHistoryVisible = false;

  /** @type {Command[]} */
  const commands = [
    {
      cmd: ':c',
      description: 'Create Document Context',
      action: 'createDocumentContext',
      shortcut: '⌘C',
    },
    { cmd: ':u', description: 'Toggle history sidebar', action: 'toggleHistory', shortcut: '⌘U' },
    { cmd: ':w', description: 'Save file', action: 'save', shortcut: '⌘S' },
    { cmd: ':wq', description: 'Save and close file', action: 'saveAndClose', shortcut: '⌘W' },
    { cmd: ':q', description: 'Close file', action: 'close', shortcut: '⌘Q' },
    { cmd: ':b', description: 'Toggle Sidebar', action: 'sidebar', shortcut: '⌘B' },
    { cmd: ':ai', description: 'Toggle AI Chat', action: 'aichat', shortcut: '⌘A' },
  ];

  const quickCommands = [
    { cmd: ':s', label: 'Analyze', icon: '⌘S' },
    { cmd: ':b', label: 'Sidebar', icon: '⌘B' },
    { cmd: ':u', label: 'History', icon: '⌘U' },
    { cmd: ':w', label: 'Save', icon: '⌘S' },
    { cmd: ':ai', label: 'AI', icon: '⌘c' },
  ];

  let isCommandMode = $state(false);
  let input = $state('');
  let selectedIndex = $state(0);
  let inputRef = $state(/** @type {HTMLInputElement | null} */ (null));

  let suggestions = $derived.by(() => {
    if (input === ':') {
      return commands;
    }
    if (input.length > 1) {
      return commands.filter(
        (cmd) =>
          cmd.cmd.startsWith(input) ||
          cmd.description.toLowerCase().includes(input.slice(1).toLowerCase())
      );
    }
    return [];
  });

  $effect(() => {
    if (suggestions.length > 0 && selectedIndex >= suggestions.length) {
      selectedIndex = 0;
    }
  });

  /**
   * GLOBAL KEYDOWN HANDLER (Capture Phase)
   * Using { capture: trues} ensures we see the event before the editor (Milkdown/ProseMirror)
   * can swallow it. This fixes the Cmd+B conflict.
   */
  $effect(() => {
    /**
     * @param {KeyboardEvent} e
     */
    const handleCaptureKeyDown = (e) => {
      // 1. Handle Escape globally
      if (e.key === 'Escape') {
        if (appState.ui.isChatOpen) {
          appState.toggleAiChat(false);
          e.preventDefault();
          return;
        }
        if (isCommandMode) {
          isCommandMode = false;
          input = '';
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
      if (e.key === 'i' && !editorState.editMode && !isCommandMode) {
        editorState.setEditMode(true);
        editorState?.editor?.commands?.focus();
        return;
      }

      const isShortcut = e.metaKey || e.ctrlKey;
      if (isShortcut && !isCommandMode && document.activeElement?.tagName !== 'INPUT') {
        const keyMap = {
          s: ':s',
          u: ':u',
          s: ':w',
          w: ':wq',
          q: ':q',
          b: ':b',
          c: ':ai',
        };

        const command = keyMap[e.key.toLowerCase()];
        if (command) {
          e.preventDefault();
          e.stopPropagation(); // Stop editor from seeing this event
          executeCommand(command);
        }
      }
    };

    window.addEventListener('keydown', handleCaptureKeyDown, { capture: true });

    return () => {
      window.removeEventListener('keydown', handleCaptureKeyDown, { capture: true });
    };
  });

  $effect(() => {
    if (isCommandMode && inputRef) {
      tick().then(() => {
        inputRef?.focus();
      });
    }
  });

  /**
   * @param {string} cmd
   */
  function executeCommand(cmd) {
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
        if (fileName) {
          onSave();
        }
        break;
      case 'close':
        goto(resolve('/'));
        break;
      case 'saveAndClose':
        if (fileName) {
          onSave();
          goto(resolve('/'));
        }
        break;
    }

    isCommandMode = false;
    input = '';
  }

  /**
   * @param {string} cmd
   */
  function handleQuickCommand(cmd) {
    executeCommand(cmd);
  }

  /**
   * @param {SubmitEvent} e
   */
  function handleSubmit(e) {
    e.preventDefault();
    if (suggestions.length > 0) {
      executeCommand(suggestions[selectedIndex].cmd);
    } else {
      executeCommand(input);
    }
  }

  /**
   * @param {KeyboardEvent} e
   */
  function handleInputKeyDown(e) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (suggestions.length > 0) {
        selectedIndex = (selectedIndex + 1) % suggestions.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (suggestions.length > 0) {
        selectedIndex = (selectedIndex - 1 + suggestions.length) % suggestions.length;
      }
    } else if (e.key === 'Tab') {
      e.preventDefault();
      if (suggestions.length > 0) {
        input = suggestions[selectedIndex].cmd;
      }
    }
  }

  let blurTimeout = $state(/** @type {NodeJS.Timeout | null} */ (null));

  function handleBlur() {
    blurTimeout = setTimeout(() => {
      isCommandMode = false;
      input = '';
    }, BLUR_DELAY);
  }

  function handleFocus() {
    if (blurTimeout !== null) {
      clearTimeout(blurTimeout);
      blurTimeout = null;
    }
  }
</script>

{#if isCommandMode && suggestions.length > 0}
  <div
    class="mx-auto max-w-2xl bg-background border border-border rounded-t-lg shadow-lg"
    role="listbox"
  >
    <div class="max-h-48 overflow-y-auto">
      {#each suggestions as suggestion, index (suggestion.cmd)}
        <button
          onclick={() => executeCommand(suggestion.cmd)}
          onfocus={handleFocus}
          class="w-full px-4 py-2 text-left text-sm font-mono flex items-center justify-between transition-colors"
          class:bg-muted={index === selectedIndex}
          class:text-foreground={index === selectedIndex}
          class:text-muted-foreground={index !== selectedIndex}
          role="option"
          aria-selected={index === selectedIndex}
        >
          <span class="font-semibold">{suggestion.cmd}</span>
          <span class="text-xs">{suggestion.description}</span>
        </button>
      {/each}
    </div>
  </div>
{/if}

<div class="bg-background border-t border-border">
  <div class="flex items-center px-4 py-2">
    <div class="w-full flex items-center gap-4 text-xs font-mono text-muted-foreground">
      <div class="flex items-center gap-1">
        <AiSparkleIcon class={cn('size-3', isThinking ? 'animate-pulse' : '')} />
        <span
          class:text-green-500={isHistoryVisible}
          class:text-muted-foreground={!isHistoryVisible}
        >
          {isHistoryVisible ? 'HISTORY' : `Version ${data?.currentVersion ?? 0} `}
        </span>
      </div>

      <div class="items-center gap-2 hidden md:flex">
        {#each quickCommands as qCmd (qCmd.cmd)}
          <button
            onclick={() => handleQuickCommand(qCmd.cmd)}
            class="group flex items-center gap-1.5 px-2 py-0.5 text-xs font-mono bg-muted/50 hover:bg-muted border border-border/50 rounded transition-all hover:border-primary/50"
          >
            <span class="text-foreground group-hover:text-primary transition-colors"
              >{qCmd.label}</span
            >
            <span
              class="text-[10px] text-muted-foreground/70 group-hover:text-primary/70 transition-colors"
              >{qCmd.icon}</span
            >
          </button>
        {/each}
      </div>
    </div>

    {#if isCommandMode}
      <form onsubmit={handleSubmit} class="flex-1 ml-4">
        <input
          bind:this={inputRef}
          type="text"
          bind:value={input}
          onkeydown={handleInputKeyDown}
          onblur={handleBlur}
          onfocus={handleFocus}
          placeholder="Enter command..."
          class="w-full bg-transparent font-mono text-sm outline-none text-foreground"
          aria-label="Command input"
        />
      </form>
    {/if}

    <DropdownMenu.Root class="dark">
      <DropdownMenu.Trigger>
        {#snippet child({ props })}
          <Button {...props} variant="ghost" size="sm" class="md:hidden">
            <MenuIcon class="size-3" />
          </Button>
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content class="dark w-56" align="start">
        <DropdownMenu.Group>
          {#each quickCommands as qCmd (qCmd.cmd)}
            <DropdownMenu.Item onclick={() => handleQuickCommand(qCmd.cmd)}>
              <span class="font-mono text-sm">{qCmd.label}</span>
              <DropdownMenu.Shortcut>{qCmd.icon}</DropdownMenu.Shortcut>
            </DropdownMenu.Item>
          {/each}
        </DropdownMenu.Group>
      </DropdownMenu.Content>
    </DropdownMenu.Root>

    <div class="ml-auto text-xs font-mono text-muted-foreground md:flex items-center gap-2 hidden">
      <kbd class="px-1.5 py-0.5 bg-muted rounded text-xs">:</kbd> command
      <span class="mx-2">|</span>
      <kbd class="px-1.5 py-0.5 bg-muted rounded text-xs">ESC</kbd> cancel
    </div>
  </div>
</div>
