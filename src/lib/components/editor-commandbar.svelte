<script>
  import { tick } from 'svelte';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import {cn} from "$lib/utils"
  import { fileManager } from '@/runes/fs.svelte.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { invalidateAll } from '$app/navigation';
  import { llmManager } from '@/runes/llm.svelte.js';
  import AiSparkleIcon from "@lucide/svelte/icons/sparkles"

  /**
   * @type {{fileName?:string ,body?: string , currentVersion?: number}}
   */
  let data = $props();

  let fileName = $derived(data.fileName);

  let isThinking = $derived(llmManager.isLoading);

  /**
   * @description Handles the save action for the editor content.
   */
  function onSave() {
    if (editorState.editor) {
      let content = editorState.editor.getHTML();
      if (fileName) {
        fileManager.createNewFile(fileName, content);
        invalidateAll();
      }
    }
  }

  /**
   * @typedef {Object} Command
   * @property {string} cmd - The command string (e.g., ":w").
   * @property {string} description - A brief description of the command.
   * @property {string} action - An identifier for the action to take.
   */

  /**
   * @type {boolean}
   */
  let isHistoryVisible = false;

  /** @type {Command[]} */
  const commands = [
    { cmd: ':u', description: 'Toggle history sidebar', action: 'toggleHistory' },
    { cmd: ':w', description: 'Save file', action: 'save' },
    { cmd: ':q', description: 'Close file', action: 'close' },
    { cmd: ':b', description: 'Toggle Sidebar', action: 'sidebar' },
    { cmd: ':ai', description: 'Toggle AI', action: 'aichat' },
  ];

  const quickCommands = [
    { cmd: ':b', label: 'Sidebar', icon: '⌘B' },
    { cmd: ':u', label: 'History', icon: '⌘U' },
    { cmd: ':w', label: 'Save', icon: '⌘S' },
    { cmd: ':q', label: 'Close', icon: '⌘Q' },
  ];

  let isCommandMode = $state(false);
  let input = $state('');

  /** @type {Command[]} */
  let suggestions = $state([]);
  let selectedIndex = $state(0);

  /** @type {HTMLInputElement | null} */
  let inputRef = null;

  $effect(() => {
    if (input.length > 1) {
      suggestions = commands.filter(
        (cmd) =>
          cmd.cmd.startsWith(input) ||
          cmd.description.toLowerCase().includes(input.slice(1).toLowerCase())
      );
      selectedIndex = 0;
    } else {
      suggestions = [];
    }
  });

  /**
   * @param {KeyboardEvent} e
   */
  const handleGlobalKeyDown = (e) => {
    if (!editorState.editMode && e.key === ':' && !isCommandMode && document.activeElement?.tagName !== 'INPUT') {
      e.preventDefault();
      isCommandMode = true;
      input = ':';
      return;
    }

    if (e.key === "i" && !editorState.editMode){
      editorState.setEditMode(true);
      editorState?.editor?.commands?.focus();
    }

    if(e.key === "Escape"){
      editorState.setEditMode(false);
      editorState?.editor?.commands?.blur();
    }

    if (e.key === 'Escape' && isCommandMode) {
      e.preventDefault();
      isCommandMode = false;
      input = '';
      return;
    }

    const isShortcut = e.metaKey || e.ctrlKey;
    if (isShortcut && !isCommandMode && document.activeElement?.tagName !== 'INPUT') {
      let handled = true;
      switch (e.key.toLowerCase()) {
        case 'u':
          executeCommand(':u');
          break;
        case 's':
          executeCommand(':w');
          break;
        case 'q':
          executeCommand(':q');
          break;
        case 'w':
          executeCommand(':wq');
          break;
        default:
          handled = false;
      }

      if (handled) {
        e.preventDefault();
      }
    }
  };

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
      case 'edit':
        console.log('[Svelte] Edit file');
        break;
      case 'help':
        console.log('[Svelte] Show help');
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

  function handleBlur() {
    setTimeout(() => {
      isCommandMode = false;
      input = '';
    }, 200);
  }
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

{#if isCommandMode && suggestions.length > 0}
  <div class="mx-auto max-w-2xl bg-background border border-border rounded-t-lg shadow-lg">
    <div class="max-h-48 overflow-y-auto">
      {#each suggestions as suggestion, index (suggestion.cmd)}
        <button
          onclick={() => executeCommand(suggestion.cmd)}
          class="w-full px-4 py-2 text-left text-sm font-mono flex items-center justify-between transition-colors"
          class:bg-muted={index === selectedIndex}
          class:text-foreground={index === selectedIndex}
          class:text-muted-foreground={index !== selectedIndex}
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
    <div class="flex items-center gap-4 text-xs font-mono text-muted-foreground">
      <AiSparkleIcon class={cn("size-3", isThinking ? "animate-pulse" : "")}/>
      <span class:text-green-500={isHistoryVisible} class:text-muted-foreground={!isHistoryVisible}>
        {isHistoryVisible ? 'HISTORY' : `Version ${data?.currentVersion ?? 0} `}
      </span>
      <span class="text-muted-foreground/50">|</span>
      <div class="flex items-center gap-2">
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
          placeholder="Enter command..."
          class="w-full bg-transparent font-mono text-sm outline-none text-foreground"
        />
      </form>
    {/if}

    <div class="ml-auto text-xs font-mono text-muted-foreground">
      <kbd class="px-1.5 py-0.5 bg-muted rounded text-xs">:</kbd> command
      <span class="mx-2">|</span>
      <kbd class="px-1.5 py-0.5 bg-muted rounded text-xs">ESC</kbd> cancel
    </div>
  </div>
</div>
