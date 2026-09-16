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
    <ul class="space-y-0.5">
      {#each backlinks as link (link.name)}
        <li>
          <button
            onclick={() => goto(resolve(`/${encodeURIComponent(link.name)}`))}
            title={link.name}
            class="block w-full rounded px-2 py-1.5 text-left transition-colors hover:bg-foreground/5 group"
          >
            <span class="block truncate text-[0.8125rem] text-muted-foreground group-hover:text-foreground">
              {#if link.folder}<span class="text-muted-foreground/50">{link.folder} / </span>{/if}{link.title}
            </span>
            {#if link.snippet}
              <span class="mt-0.5 line-clamp-2 text-xs text-muted-foreground/60">{link.snippet}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {:else if !loading}
    <p class="px-2 text-sm text-muted-foreground">No writings link here</p>
  {/if}
</div>
