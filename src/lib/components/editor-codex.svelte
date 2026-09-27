<script>
  /**
   * EditorCodex - codex entries the open writing mentions, most mentioned first.
   * Follows the saved text, so it catches up on each autosave.
   */
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { codexManager, openCodex } from '$lib/runes/codex.svelte.js';

  /** @type {{ fileName: string, content: string }} */
  let { fileName, content } = $props();

  let binder = $derived(fileName.includes('/') ? fileName.split('/')[0] : '');
  let entities = $derived(codexManager.entities(binder || null));
  let byPath = $derived(Object.fromEntries(entities.map((e) => [e.path, e])));

  /** @type {import('$lib/runes/codex.svelte.js').Mention[]} */
  let mentions = $state([]);

  $effect(() => {
    codexManager.load(binder || null);
  });

  $effect(() => {
    // Re-run when the codex itself changes too, not only the text.
    void entities;
    const text = content;
    const b = binder;
    let cancelled = false;
    const timer = setTimeout(() => {
      codexManager
        .detect(b || null, text)
        .then((found) => {
          if (!cancelled) mentions = found;
        })
        .catch((e) => console.error('Codex detection failed:', e));
    }, 300);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });
</script>

<div class="px-3 pb-4 font-sans">
  {#if mentions.length > 0}
    <div class="flex flex-wrap gap-1.5">
      {#each mentions as mention (mention.path)}
        {@const entity = byPath[mention.path]}
        {#if entity}
          <button
            onclick={() => openCodex({ binder, entry: entity.path })}
            title={entity.summary ? `${entity.name}: ${entity.summary}` : entity.name}
            class="max-w-full rounded-full border border-border bg-muted/30 px-2.5 py-1 text-left text-[0.75rem] text-muted-foreground transition-colors hover:border-primary/40 hover:bg-primary/10 hover:text-primary"
          >
            <span class="truncate">{entity.name}</span>
            <span class="opacity-50 tabular-nums">×{mention.count}</span>
          </button>
        {/if}
      {/each}
    </div>
  {:else}
    <p class="px-1 text-[0.75rem] text-muted-foreground/60">
      {entities.length ? 'No codex entries mentioned yet.' : 'No codex entries yet.'}
    </p>
  {/if}
  <button
    onclick={() => openCodex({ binder, new: 'character' })}
    class="mt-2 flex items-center gap-1 px-1 text-[0.75rem] text-muted-foreground/70 hover:text-foreground"
  >
    <PlusIcon strokeWidth={1.5} class="size-3" /> New entry
  </button>
</div>
