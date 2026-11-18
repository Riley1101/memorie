<script>
  import { cn } from '$lib/utils';

  /**
   * @typedef {Object} Props
   * @property {import('./slash-menu.svelte.js').SlashCommandItem[]} items
   * @property {number} selectedIndex
   * @property {(item: import('./slash-menu.svelte.js').SlashCommandItem) => void} onSelect
   */

  /** @type {Props} */
  let { items, selectedIndex, onSelect } = $props();
</script>

<div
  class="w-64 glass rounded-group overflow-hidden not-prose font-sans"
  role="listbox"
>
  <div class="max-h-72 overflow-y-auto p-1">
    {#each items as item, index (item.id)}
      <button
        type="button"
        onmousedown={(e) => e.preventDefault()}
        onclick={() => onSelect(item)}
        class={cn(
          'w-full px-3 py-2 text-left text-sm flex flex-col rounded-[6px] transition-colors',
          index === selectedIndex
            ? 'bg-selection text-foreground'
            : 'text-muted-foreground hover:bg-fill'
        )}
        role="option"
        aria-selected={index === selectedIndex}
      >
        <span class="font-medium text-foreground">{item.label}</span>
        <span class="text-xs opacity-70">{item.description}</span>
      </button>
    {:else}
      <div class="px-3 py-2 text-sm text-muted-foreground">No matches</div>
    {/each}
  </div>
</div>
