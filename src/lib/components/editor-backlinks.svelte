<script>
  /**
   * EditorBacklinks - writings that reference the open one through `[[` links.
   */
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { findBacklinks } from '$lib/components/plugins/doc-ref.svelte.js';

  /** @type {{ fileName: string }} */
  let { fileName } = $props();

  /** @type {import('$lib/components/plugins/doc-ref.svelte.js').Backlink[]} */
  let backlinks = $state([]);
  let loading = $state(true);

  // Rescan only when some *other* writing changes; autosaving this one leaves it untouched.
  let othersSignature = $derived(
    fileManager.files
      .filter((f) => f.name !== fileName)
      .map((f) => `${f.name}:${f.last_modified}`)
      .join('|')
  );

  $effect(() => {
    void othersSignature;
    const target = fileName;
    let cancelled = false;
    findBacklinks(target)
      .then((found) => {
        if (!cancelled) backlinks = found;
      })
      .catch((e) => {
        console.error('Backlink scan failed:', e);
        if (!cancelled) backlinks = [];
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
    };
  });
</script>

<div class="px-3 pb-4">
  {#if backlinks.length > 0}
    <div class="flex flex-wrap gap-1.5">
      {#each backlinks as link (link.name)}
        <button
          onclick={() => goto(resolve(`/${encodeURIComponent(link.name)}`))}
          title={link.snippet ? `${link.name}\n${link.snippet}` : link.name}
          class="max-w-full rounded-full border border-border bg-muted/30 px-2.5 py-1 text-left text-[0.75rem] text-muted-foreground transition-colors hover:border-primary/40 hover:bg-primary/10 hover:text-primary"
        >
          <span class="truncate block">
            {#if link.folder}<span class="text-muted-foreground/50">{link.folder} / </span>{/if}{link.title}
          </span>
        </button>
      {/each}
    </div>
  {:else if !loading}
    <p class="px-2 text-sm text-muted-foreground">No writings link here</p>
  {/if}
</div>
