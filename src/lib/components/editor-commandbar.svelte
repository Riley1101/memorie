<script>
  import { onMount, onDestroy, tick } from 'svelte'
  import { fileManager } from '@/runes/fs.svelte.js';

  /**
   * @type {{fileName?:string ,body?: string }}
   */
  let data = $props();

  let fileName = $derived(data.fileName);
  let content = $derived(data.body);

  /**
   * @typedef {Object} Command
   * @property {string} cmd - The command string (e.g., ":w").
   * @property {string} description - A brief description of the command.
   * @property {string} action - An identifier for the action to take.
   */

  /**
   * @type {() => void}
   */
  let onToggleHistory = ()=>{};
  /**
   * @type {boolean}
   */
  let isHistoryVisible = false;

  /** @type {Command[]} */
  const commands = [
    { cmd: ':history', description: 'Toggle history sidebar', action: 'toggleHistory' },
    { cmd: ':w', description: 'Save file', action: 'save' },
    { cmd: ':q', description: 'Close file', action: 'close' },
    { cmd: ':wq', description: 'Save and close', action: 'saveAndClose' },
    { cmd: ':e', description: 'Edit file', action: 'edit' },
    { cmd: ':help', description: 'Show help', action: 'help' },
  ]

  const quickCommands = [
    { cmd: ':history', label: 'History', icon: '⌘H' },
    { cmd: ':w', label: 'Save', icon: '⌘S' },
    { cmd: ':q', label: 'Close', icon: '⌘Q' },
    { cmd: ':wq', label: 'Save & Close', icon: '⌘W' },
  ]

  // --- State ---
  let isCommandMode = $state(false)
  let input = $state('')
  /** @type {Command[]} */
  let suggestions = $state([])
  let selectedIndex = $state(0)
  /** @type {HTMLInputElement | null} */
  let inputRef = null

  $effect(()=>{
    if (input.length > 1) {
      suggestions = commands.filter(
        (cmd) =>
          cmd.cmd.startsWith(input) ||
          cmd.description.toLowerCase().includes(input.slice(1).toLowerCase()),
      )
      selectedIndex = 0
    } else {
      suggestions = []
    }
  })

  /**
   * @param {KeyboardEvent} e
   */
  const handleGlobalKeyDown = (e) => {
    if (e.key === ':' && !isCommandMode && document.activeElement?.tagName !== 'INPUT') {
      e.preventDefault()
      isCommandMode = true
      input = ':'
    } else if (e.key === 'Escape' && isCommandMode) {
      isCommandMode = false
      input = ''
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleGlobalKeyDown)
  })

  onDestroy(() => {
    window.removeEventListener('keydown', handleGlobalKeyDown)
  })

  $effect(()=>{
    if (isCommandMode && inputRef) {
      tick().then(() => {
        inputRef?.focus()
      })
    }
  })

  /**
   * @param {string} cmd
   */
  function executeCommand(cmd) {
    const command = commands.find((c) => c.cmd === cmd)
    if (!command) return

    switch (command.action) {
      case 'toggleHistory':
        onToggleHistory()
        break
      case 'save':
        if (fileName) {
          let content
          fileManager.createNewFile(fileName, content);
        }
        break
      case 'close':
        console.log('[Svelte] Close file')
        break
      case 'saveAndClose':
        console.log('[Svelte] Save and close file')
        break
      case 'edit':
        console.log('[Svelte] Edit file')
        break
      case 'help':
        console.log('[Svelte] Show help')
        break
    }

    isCommandMode = false
    input = ''
  }

  /**
   * @param {string} cmd
   */
  function handleQuickCommand(cmd) {
    executeCommand(cmd)
  }

  function handleSubmit(e) {
    e.preventDefault();
    if (suggestions.length > 0) {
      executeCommand(suggestions[selectedIndex].cmd)
    } else {
      executeCommand(input)
    }
  }

  /**
   * @param {KeyboardEvent} e
   */
  function handleInputKeyDown(e) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      if (suggestions.length > 0) {
        selectedIndex = (selectedIndex + 1) % suggestions.length
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      if (suggestions.length > 0) {
        selectedIndex = (selectedIndex - 1 + suggestions.length) % suggestions.length
      }
    } else if (e.key === 'Tab') {
      e.preventDefault()
      if (suggestions.length > 0) {
        input = suggestions[selectedIndex].cmd
      }
    }
  }

  function handleBlur() {
    setTimeout(() => {
      isCommandMode = false
      input = ''
    }, 200)
  }
</script>

{#if isCommandMode && suggestions.length > 0}
  <div
    class="mx-auto max-w-2xl bg-background border border-border rounded-t-lg shadow-lg"
  >
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
      <span
        class:text-green-500={isHistoryVisible}
        class:text-muted-foreground={!isHistoryVisible}
      >
        {isHistoryVisible ? 'HISTORY' : 'NO HISTORY'}
      </span>
      <span>NORMAL</span>

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