<script>
  import { invoke } from '@tauri-apps/api/core';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { SvelteMap } from 'svelte/reactivity';
  import { fileManager, baseOf } from '$lib/runes/fs.svelte.js';
  import { buildFileTree, formatFileName } from '$lib/utils.js';
  import { parseWriting, serializeWriting } from '$lib/front-matter.js';
  import { countWords, markdownToPlainText } from '$lib/runes/writing.svelte.js';
  import { toast } from '$lib/toast.js';
  import PencilIcon from '@lucide/svelte/icons/pencil';

  /** @type {{ binder: string }} */
  let { binder } = $props();

  /**
   * Parsed writings by name, refreshed when a file's modification time changes.
   * @type {SvelteMap<string, { modified: number, meta: import('$lib/front-matter.js').SceneMeta, extra: string[], body: string, original: { meta: import('$lib/front-matter.js').SceneMeta, raw: string }, excerpt: string, words: number }>}
   */
  const cache = new SvelteMap();
  let loading = $state(true);

  const format = new Intl.NumberFormat();

  let files = $derived(fileManager.filesUnder(binder));

  $effect(() => {
    const stale = files.filter((f) => cache.get(f.name)?.modified !== f.last_modified);
    if (stale.length === 0) {
      loading = false;
      return;
    }
    let cancelled = false;
    invoke('read_files', { names: stale.map((f) => f.name) })
      .then((/** @type {[string, { Ok?: string } | string][]} */ results) => {
        if (cancelled) return;
        const modified = new Map(stale.map((f) => [f.name, f.last_modified]));
        for (const [name, result] of results) {
          const content = typeof result === 'string' ? result : result?.Ok;
          if (typeof content !== 'string') continue;
          const { meta, extra, body, raw } = parseWriting(content);
          const plain = markdownToPlainText(body).replace(/\s+/g, ' ').trim();
          cache.set(name, {
            modified: modified.get(name) ?? 0,
            meta,
            extra,
            body,
            original: { meta, raw },
            excerpt: plain.length > 220 ? `${plain.slice(0, 220)}…` : plain,
            words: countWords(plain),
          });
        }
      })
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
    const tree = buildFileTree(fileManager.files, binder, fileManager.folders);
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
      await fileManager.saveFile(name, serializeWriting(meta, entry.extra, entry.body, entry.original));
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
    if (/revis/.test(s)) return 'bg-primary/15 text-primary';
    if (/draft/.test(s)) return 'bg-warning/20 text-foreground';
    return 'bg-muted text-muted-foreground';
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
              : 'border-border/60 text-muted-foreground hover:text-foreground hover:bg-muted/40'}"
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
              class="group relative flex flex-col min-h-40 rounded-lg border border-border/60 bg-muted/10 p-3 transition-colors hover:border-border hover:bg-muted/25"
            >
              <div class="flex items-start gap-2">
                <a
                  href={resolve(`/${encodeURIComponent(file.name)}`)}
                  onclick={(e) => {
                    e.preventDefault();
                    goto(resolve(`/${encodeURIComponent(file.name)}`));
                  }}
                  class="flex-1 min-w-0 text-base leading-snug hover:underline underline-offset-2"
                >
                  {formatFileName(baseOf(file.name))}
                </a>
                {#if card?.meta.status}
                  <span class="shrink-0 rounded-full px-2 py-0.5 font-sans text-[0.6875rem] {statusTone(card.meta.status)}">
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
                  class="mt-2 flex-1 w-full resize-none rounded-md border border-border bg-background px-2 py-1.5 font-sans text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                ></textarea>
              {:else}
                <button
                  type="button"
                  onclick={() => startEdit(file.name)}
                  title="Edit synopsis"
                  class="mt-2 flex-1 text-left font-sans text-sm leading-snug rounded-md -mx-1 px-1 hover:bg-muted/40"
                >
                  {#if !card && loading}
                    <span class="text-muted-foreground/50">…</span>
                  {:else if card?.meta.synopsis}
                    <span class="line-clamp-5 whitespace-pre-line">{card.meta.synopsis}</span>
                  {:else if card?.excerpt}
                    <span class="line-clamp-4 italic text-muted-foreground/70">{card.excerpt}</span>
                  {:else}
                    <span class="text-muted-foreground/50">Add a synopsis</span>
                  {/if}
                </button>
              {/if}

              <div class="mt-2 flex items-center justify-between font-sans text-[0.6875rem] text-muted-foreground/60 tabular-nums">
                <span>{card ? `${format.format(card.words)} words` : ''}</span>
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
    <p class="py-16 text-center text-sm text-muted-foreground">No writings with this status.</p>
  {/if}
</div>
