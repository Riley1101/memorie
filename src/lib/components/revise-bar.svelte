<script>
  import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import { reviseState } from '$lib/runes/revise.svelte.js';
  import { goToChange, stopRevise } from '$lib/components/plugins/revise.js';

  /**
   * Floats above the page while it is compared with an earlier version: what
   * it is compared with, how many changes, a way through them, and Done.
   */

  let summary = $derived(
    reviseState.count === 0
      ? 'No changes'
      : reviseState.position
        ? `${reviseState.position} of ${reviseState.count}`
        : reviseState.count === 1
          ? '1 change'
          : `${reviseState.count} changes`
  );
</script>

{#if reviseState.open}
  <div class="absolute inset-x-0 top-3 z-30 flex justify-center px-4 pointer-events-none">
    <div
      role="toolbar"
      aria-label="Comparing versions"
      class="glass pointer-events-auto flex items-center gap-1 h-10 max-w-full pl-4 pr-1 rounded-full text-meta tabular-nums text-muted-foreground"
    >
      <span class="truncate">Changes since {reviseState.label}</span>
      <span class="h-3.5 w-px mx-2 bg-hair shrink-0" aria-hidden="true"></span>
      <span class="shrink-0 text-foreground" aria-live="polite">{summary}</span>
      <button
        type="button"
        onclick={() => goToChange(-1)}
        disabled={reviseState.count === 0}
        aria-label="Previous change"
        class="ml-1 flex items-center justify-center size-8 shrink-0 rounded-full hover:bg-fill hover:text-foreground transition-colors disabled:opacity-40 disabled:pointer-events-none"
      >
        <ChevronUpIcon strokeWidth={1.5} class="size-4" />
      </button>
      <button
        type="button"
        onclick={() => goToChange(1)}
        disabled={reviseState.count === 0}
        aria-label="Next change"
        class="flex items-center justify-center size-8 shrink-0 rounded-full hover:bg-fill hover:text-foreground transition-colors disabled:opacity-40 disabled:pointer-events-none"
      >
        <ChevronDownIcon strokeWidth={1.5} class="size-4" />
      </button>
      <button
        type="button"
        onclick={stopRevise}
        title="Stop comparing (Esc)"
        class="ml-1 flex items-center h-8 px-3.5 shrink-0 rounded-full bg-foreground/6 text-foreground hover:bg-fill transition-colors"
      >
        Done
      </button>
    </div>
  </div>
{/if}
