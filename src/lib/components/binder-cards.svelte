<script>
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { sceneCache } from '$lib/runes/scenes.svelte.js';
  import { compareByOrder } from '$lib/runes/ordering.svelte.js';
  import { naturalCollator } from '$lib/utils.js';
  import { serializeWriting } from '$lib/front-matter.js';
  import { fileManager, baseOf } from '$lib/runes/fs.svelte.js';
  import { buildFileTree, formatFileName, formatDate } from '$lib/utils.js';
  import { toast } from '$lib/toast.js';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import { Button } from '$lib/components/ui/button/index.js';

  /** @type {{ binder: string }} */
  let { binder } = $props();

  // Shared with the tree view, so a status set here shows up there too.
  const cache = sceneCache.entries;
  let loading = $state(true);

  const format = new Intl.NumberFormat();

  let files = $derived(fileManager.filesUnder(binder));

  $effect(() => {
    const wanted = files;
    let cancelled = false;
    sceneCache
      .load(wanted)
      .catch((e) => toast.error('Could not load cards', e))
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  /**
   * Folders in tree order, each with the writings directly inside it.
   * @type {{ path: string, depth: number, files: { name: string }[] }[]}
   */
  let sections = $derived.by(() => {
    const tree = buildFileTree(fileManager.files, binder, fileManager.folders, (a, b) => compareByOrder(a.name, b.name, (x, y) => naturalCollator.compare(baseOf(x), baseOf(y))));
    /** @type {{ path: string, depth: number, files: { name: string }[] }[]} */
    const out = [];
    const walk = (/** @type {any[]} */ nodes, /** @type {string[]} */ path) => {
      const own = nodes.filter((n) => n.type === 'file').map((n) => n.file);
      if (own.length || path.length === 0) out.push({ path: path.join(' / '), depth: path.length, files: own });
      for (const node of nodes) {
        if (node.type === 'folder') walk(node.children, [...path, node.name]);
      }
    };
    walk(tree, []);
    return out;
  });

  let statuses = $derived(
    [...new Set([...cache.values()].map((c) => c.meta.status).filter(Boolean))].sort()
  );

  /** '' shows everything; '—' shows writings with no status. */
  let statusFilter = $state('');

  /** @param {string} name */
  function visible(name) {
    if (!statusFilter) return true;
    const status = cache.get(name)?.meta.status;
    return statusFilter === '—' ? !status : status === statusFilter;
  }

  let editing = $state(/** @type {string | null} */ (null));
  let draftSynopsis = $state('');

  /** @param {string} name */
  function startEdit(name) {
    editing = name;
    draftSynopsis = cache.get(name)?.meta.synopsis ?? '';
  }

  /** @param {string} name */
  async function saveSynopsis(name) {
    if (editing !== name) return;
    editing = null;
    const entry = cache.get(name);
    if (!entry) return;
    const synopsis = draftSynopsis.trim();
    if (synopsis === (entry.meta.synopsis ?? '')) return;
    const meta = { ...entry.meta, synopsis: synopsis || undefined };
    try {
      await fileManager.saveFile(
        name,
        // `original` lets the serializer leave untouched front matter exactly
        // as it was on disk; the synopsis changed, so it has to be rewritten.
        serializeWriting(meta, entry.extra, entry.body, { meta: entry.meta, raw: entry.raw })
      );
      // Show it right away; the refreshed file list will re-read the file.
      cache.set(name, { ...entry, meta });
    } catch (e) {
      toast.error('Could not save synopsis', e);
    }
  }

  /** @param {string} status */
  function statusTone(status) {
    const s = status.toLowerCase();
    if (/done|final|complete/.test(s)) return 'bg-success/15 text-success';
    if (/draft/.test(s)) return 'bg-warning/20 text-foreground';
    return 'bg-primary/15 text-primary';
  }
</script>

<div class="pb-20 pr-2">
  {#if statuses.length}
    <div class="flex flex-wrap items-center gap-1.5 mb-4 font-sans" role="group" aria-label="Filter by status">
      {#each ['', ...statuses, '—'] as option (option)}
        <button
          type="button"
          onclick={() => (statusFilter = option)}
          aria-pressed={statusFilter === option}
          class="h-7 px-2.5 rounded-full text-xs transition-colors border
            {statusFilter === option
              ? 'border-primary bg-primary/10 text-primary'
              : 'border-hair text-muted-foreground hover:text-foreground hover:bg-fill'}"
        >
          {option === '' ? 'All' : option === '—' ? 'No status' : option}
        </button>
      {/each}
    </div>
  {/if}

  {#each sections as section (section.path)}
    {@const shown = section.files.filter((f) => visible(f.name))}
    {#if shown.length}
      <section class="mb-8">
        {#if section.path}
          <h3 class="mb-3 text-sm font-medium text-muted-foreground">{section.path}</h3>
        {/if}
        <ul class="grid gap-3 grid-cols-[repeat(auto-fill,minmax(14rem,1fr))]">
          {#each shown as file (file.name)}
            {@const card = cache.get(file.name)}
            <li
              class="group relative flex flex-col min-h-40 rounded-group bg-card p-3 shadow-[0_0_0_1px_var(--hair)] transition-shadow hover:shadow-[0_0_0_1px_var(--hair),0_6px_18px_-8px_rgb(0_0_0/0.25)]"
            >
              <div class="flex items-start gap-2">
                <a
                  href={resolve(`/${encodeURIComponent(file.name)}`)}
                  onclick={(e) => {
                    e.preventDefault();
                    goto(resolve(`/${encodeURIComponent(file.name)}`));
                  }}
                  class="flex-1 min-w-0 text-[13.5px] font-medium leading-snug hover:underline underline-offset-2"
                >
                  {formatFileName(baseOf(file.name))}
                </a>
                {#if card?.meta.status}
                  <span class="shrink-0 rounded-full px-2 py-0.5 text-xs {statusTone(card.meta.status)}">
                    {card.meta.status}
                  </span>
                {/if}
              </div>

              {#if editing === file.name}
                <!-- svelte-ignore a11y_autofocus -->
                <textarea
                  bind:value={draftSynopsis}
                  autofocus
                  rows="4"
                  aria-label="Synopsis"
                  placeholder="What happens here"
                  onblur={() => saveSynopsis(file.name)}
                  onkeydown={(e) => {
                    if (e.key === 'Escape') {
                      editing = null;
                    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                      e.currentTarget.blur();
                    }
                  }}
                  class="mt-2 flex-1 w-full resize-none rounded-field border border-hair bg-paper px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                ></textarea>
              {:else}
                <button
                  type="button"
                  onclick={() => startEdit(file.name)}
                  title="Edit synopsis"
                  class="mt-2 flex-1 text-left font-serif text-sm leading-snug rounded-[6px] -mx-1 px-1 hover:bg-fill"
                >
                  {#if !card && loading}
                    <span class="text-muted-foreground">…</span>
                  {:else if card?.meta.synopsis}
                    <span class="line-clamp-5 whitespace-pre-line">{card.meta.synopsis}</span>
                  {:else if card?.excerpt}
                    <span class="line-clamp-4 italic text-muted-foreground">{card.excerpt}</span>
                  {:else}
                    <span class="text-muted-foreground">Add a synopsis</span>
                  {/if}
                </button>
              {/if}

              <div class="mt-2 flex items-center justify-between meta">
                <span class="flex items-center gap-1.5">
                  {#if card}
                    <span>{formatDate(card.modified)}</span>
                    <span class="opacity-30">|</span>
                    <span>{format.format(card.words)} words</span>
                  {/if}
                </span>
                {#if editing !== file.name}
                  <PencilIcon strokeWidth={1.5} class="size-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/each}

  {#if !loading && statusFilter && sections.every((s) => !s.files.some((f) => visible(f.name)))}
    <div class="flex flex-col items-center gap-3 py-16 text-center">
      <p class="text-sm text-muted-foreground">No writings are marked “{statusFilter === '—' ? 'no status' : statusFilter}”.</p>
      <Button variant="outline" size="sm" onclick={() => (statusFilter = '')} disabled={false}>Clear filter</Button>
    </div>
  {/if}
</div>
