<script>
  import { cn } from '$lib/utils';

  /**
   * @typedef {Object} Props
   * @property {import('./doc-ref.svelte.js').DocRefItem[]} items
   * @property {number} selectedIndex
   * @property {(item: import('./doc-ref.svelte.js').DocRefItem) => void} onSelect
   */

  /** @type {Props} */
  let { items, selectedIndex, onSelect } = $props();
</script>

<div
  class="w-72 bg-popover border border-border rounded-xl shadow-lg overflow-hidden not-prose"
  role="listbox"
>
  <div class="max-h-72 overflow-y-auto p-1">
    {#each items as item, index (item.name)}
      <button
        type="button"
        onmousedown={(e) => e.preventDefault()}
        onclick={() => onSelect(item)}
        class={cn(
          'w-full px-3 py-2 text-left text-sm flex flex-col rounded-sm transition-colors',
          index === selectedIndex
            ? 'bg-accent text-accent-foreground'
            : 'text-muted-foreground hover:bg-muted'
        )}
        role="option"
        aria-selected={index === selectedIndex}
      >
        <span class="font-medium text-foreground truncate">{item.title}</span>
        {#if item.folder}
          <span class="text-xs opacity-70 truncate">{item.folder}</span>
        {/if}
      </button>
    {:else}
      <div class="px-3 py-2 text-sm text-muted-foreground">No writings found</div>
    {/each}
  </div>
</div>
