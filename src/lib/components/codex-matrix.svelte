<script>
  /**
   * CodexMatrix - which codex entries each writing in a binder mentions, as a
   * heatmap (entries × writings, in binder order), plus the pairs of entries
   * that most often share a writing.
   */
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { KINDS } from '$lib/runes/codex.svelte.js';
  import { buildFileTree, formatFileName } from '$lib/utils.js';

  /**
   * @typedef {import('$lib/runes/codex.svelte.js').Entity} Entity
   * @typedef {import('$lib/runes/codex.svelte.js').Matrix} Matrix
   * @typedef {{ name: string, label: string, chapter: string }} Scene
   */

  /**
   * @type {{
   *   binder: string,
   *   matrix: Matrix | null,
   *   loading: boolean,
   *   onopen: (entity: Entity) => void,
   * }}
   */
  let { binder, matrix, loading, onopen } = $props();

  /** The filter picked; null until the writer picks one. @type {import('$lib/runes/codex.svelte.js').EntityKind | 'all' | null} */
  let picked = $state(null);
  /** Until then, the first kind this binder has entries of. */
  let kind = $derived(
    picked ?? KINDS.find((k) => matrix?.entities.some((e) => e.kind === k.id))?.id ?? 'all'
  );
  /** @type {{ entity: Entity, scene: Scene, count: number, x: number, y: number } | null} */
  let hover = $state(null);

  /** The binder's writings in tree order, each with the folder it sits in. */
  let scenes = $derived.by(() => {
    /** @type {Scene[]} */
    const out = [];
    const walk = (nodes, chapter) => {
      for (const node of nodes) {
        if (node.type === 'folder') walk(node.children, node.path.join(' / '));
        else out.push({ name: node.file.name, label: formatFileName(node.name), chapter });
      }
    };
    walk(buildFileTree(fileManager.files, binder, fileManager.folders), '');
    return out;
  });

  /** Consecutive writings that share a folder, for the header above the columns. */
  let chapters = $derived.by(() => {
    /** @type {{ label: string, span: number }[]} */
    const groups = [];
    for (const scene of scenes) {
      const last = groups[groups.length - 1];
      if (last && last.label === scene.chapter) last.span += 1;
      else groups.push({ label: scene.chapter, span: 1 });
    }
    return groups;
  });

  let countsByScene = $derived(
    Object.fromEntries((matrix?.scenes ?? []).map((s) => [s.name, s.counts]))
  );

  let rows = $derived(
    (matrix?.entities ?? [])
      .filter((e) => kind === 'all' || e.kind === kind)
      .map((entity) => {
        const counts = scenes.map((s) => countsByScene[s.name]?.[entity.path] ?? 0);
        return { entity, counts, total: counts.reduce((a, b) => a + b, 0), present: counts.filter(Boolean).length };
      })
  );

  /** Pairs of entries by how many writings mention both. */
  let pairs = $derived.by(() => {
    const list = [];
    for (let i = 0; i < rows.length; i++) {
      for (let j = i + 1; j < rows.length; j++) {
        let shared = 0;
        for (let k = 0; k < scenes.length; k++) {
          if (rows[i].counts[k] && rows[j].counts[k]) shared += 1;
        }
        if (!shared) continue;
        const a = rows[i].entity;
        const b = rows[j].entity;
        const declared = [
          ...a.relationships.filter((r) => r.to.toLowerCase() === b.name.toLowerCase()).map((r) => r.kind),
          ...b.relationships.filter((r) => r.to.toLowerCase() === a.name.toLowerCase()).map((r) => r.kind),
        ].filter(Boolean);
        list.push({ a, b, shared, declared });
      }
    }
    return list.sort((x, y) => y.shared - x.shared).slice(0, 12);
  });

  let maxShared = $derived(Math.max(1, ...pairs.map((p) => p.shared)));

  /** Four steps of one hue, light to dark; zero stays empty. */
  const STEPS = [
    { min: 1, label: '1', cls: 'bg-primary/20' },
    { min: 2, label: '2–3', cls: 'bg-primary/45' },
    { min: 4, label: '4–7', cls: 'bg-primary/70' },
    { min: 8, label: '8+', cls: 'bg-primary' },
  ];

  /** @param {number} n */
  function stepClass(n) {
    if (!n) return 'bg-muted/40';
    return [...STEPS].reverse().find((s) => n >= s.min)?.cls ?? STEPS[0].cls;
  }

  const CELL = 14;
  const GAP = 2;

  /** @param {MouseEvent} e @param {Entity} entity @param {Scene} scene @param {number} count */
  function showTip(e, entity, scene, count) {
    const box = /** @type {HTMLElement} */ (e.currentTarget).getBoundingClientRect();
    hover = { entity, scene, count, x: box.left + box.width / 2, y: box.top };
  }
</script>

<div class="flex flex-col gap-8 font-sans">
  <div class="flex flex-wrap items-center gap-1" role="group" aria-label="Show">
    {#each [{ id: 'all', plural: 'Everything' }, ...KINDS] as k (k.id)}
      <button
        type="button"
        onclick={() => (picked = /** @type {any} */ (k.id))}
        aria-pressed={kind === k.id}
        class="h-7 px-2.5 rounded-md border text-xs transition-colors
          {kind === k.id ? 'border-primary/40 bg-primary/10 text-primary' : 'border-border/60 text-muted-foreground hover:text-foreground'}"
      >
        {k.plural}
      </button>
    {/each}
  </div>

  {#if loading && !matrix}
    <p class="text-sm text-muted-foreground">Reading {binder}…</p>
  {:else if !rows.length}
    <p class="text-sm text-muted-foreground/70">No {kind === 'all' ? 'entries' : KINDS.find((k) => k.id === kind)?.plural.toLowerCase()} in this binder's codex yet.</p>
  {:else if !scenes.length}
    <p class="text-sm text-muted-foreground/70">{binder} has no writings yet.</p>
  {:else}
    <section class="grid gap-3">
      <div class="flex items-baseline justify-between gap-4 flex-wrap">
        <h3 class="text-[0.6875rem] font-mono uppercase tracking-wider text-metadata">Mentions per writing</h3>
        <div class="flex items-center gap-2 text-[0.6875rem] text-muted-foreground" aria-label="Legend: mentions per writing">
          <span>Mentions</span>
          <span class="flex items-center gap-1"><span class="size-3 rounded-[3px] bg-muted/40 border border-border/40"></span>0</span>
          {#each STEPS as step (step.min)}
            <span class="flex items-center gap-1"><span class="size-3 rounded-[3px] {step.cls}"></span>{step.label}</span>
          {/each}
        </div>
      </div>

      <div class="overflow-x-auto pb-2">
        <table class="border-separate text-xs" style="border-spacing: {GAP}px">
          <thead>
            {#if chapters.some((c) => c.label)}
              <tr>
                <th class="sticky left-0 bg-background"></th>
                {#each chapters as chapter, i (i)}
                  <th colspan={chapter.span} class="p-0 text-left font-normal" title={chapter.label || binder}>
                    <!-- Fixed width, so a long folder name can't widen the columns under it. -->
                    <div
                      class="truncate border-l border-border/60 pl-1 text-[0.625rem] text-muted-foreground/70"
                      style="width: {chapter.span * (CELL + GAP) - GAP}px"
                    >
                      {chapter.label || '·'}
                    </div>
                  </th>
                {/each}
                <th></th>
              </tr>
            {/if}
          </thead>
          <tbody>
            {#each rows as row (row.entity.path)}
              <tr>
                <th scope="row" class="sticky left-0 z-10 bg-background pr-3 text-left font-normal whitespace-nowrap">
                  <button type="button" onclick={() => onopen(row.entity)} class="text-foreground hover:text-primary max-w-44 truncate block">
                    {row.entity.name}
                  </button>
                </th>
                {#each row.counts as count, k (k)}
                  <td class="p-0">
                    <button
                      type="button"
                      onclick={() => goto(resolve(`/${encodeURIComponent(scenes[k].name)}`))}
                      onmouseenter={(e) => showTip(e, row.entity, scenes[k], count)}
                      onfocus={(e) => showTip(/** @type {any} */ (e), row.entity, scenes[k], count)}
                      onmouseleave={() => (hover = null)}
                      onblur={() => (hover = null)}
                      aria-label="{row.entity.name} in {scenes[k].label}: {count} mention{count === 1 ? '' : 's'}"
                      tabindex={count ? 0 : -1}
                      class="block rounded-[3px] hover:ring-2 hover:ring-ring/60 focus-visible:ring-2 focus-visible:ring-ring outline-none {stepClass(count)}"
                      style="width: {CELL}px; height: {CELL}px"
                    ></button>
                  </td>
                {/each}
                <td class="pl-2 whitespace-nowrap text-muted-foreground tabular-nums">
                  {row.present}/{scenes.length}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="text-xs text-muted-foreground/70">
        Columns are writings in binder order. The number on the right is how many writings mention the entry. Click a square to open the writing.
      </p>
    </section>

    {#if kind !== 'all' || rows.length > 1}
      <section class="grid gap-3 max-w-2xl">
        <h3 class="text-[0.6875rem] font-mono uppercase tracking-wider text-metadata">Often together</h3>
        {#if pairs.length}
          <table class="w-full text-sm">
            <thead class="sr-only">
              <tr><th>Pair</th><th>Writings shared</th><th>Relationship</th></tr>
            </thead>
            <tbody>
              {#each pairs as pair (pair.a.path + pair.b.path)}
                <tr class="border-b border-border/30 last:border-0">
                  <td class="py-1.5 pr-3 whitespace-nowrap">
                    <button type="button" class="hover:text-primary" onclick={() => onopen(pair.a)}>{pair.a.name}</button>
                    <span class="text-muted-foreground/60">&amp;</span>
                    <button type="button" class="hover:text-primary" onclick={() => onopen(pair.b)}>{pair.b.name}</button>
                  </td>
                  <td class="py-1.5 w-full">
                    <div class="flex items-center gap-2">
                      <div class="h-2 rounded-r-[4px] bg-primary/70" style="width: {(pair.shared / maxShared) * 100}%"></div>
                      <span class="text-xs text-muted-foreground tabular-nums whitespace-nowrap">{pair.shared}</span>
                    </div>
                  </td>
                  <td class="py-1.5 pl-3 text-xs text-muted-foreground whitespace-nowrap">
                    {pair.declared.join(', ') || '—'}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          <p class="text-xs text-muted-foreground/70">Bars count writings that mention both. The right column shows relationships set on their sheets.</p>
        {:else}
          <p class="text-sm text-muted-foreground/70">No two entries share a writing yet.</p>
        {/if}
      </section>
    {/if}
  {/if}
</div>

{#if hover}
  <div
    role="tooltip"
    class="fixed z-50 -translate-x-1/2 -translate-y-full -mt-2 pointer-events-none rounded-md border border-border bg-popover px-2.5 py-1.5 text-xs text-popover-foreground shadow-md"
    style="left: {hover.x}px; top: {hover.y - 6}px"
  >
    <div class="font-medium">{hover.entity.name}</div>
    <div class="text-muted-foreground">
      {hover.scene.chapter ? `${hover.scene.chapter} / ` : ''}{hover.scene.label}
    </div>
    <div class="tabular-nums">{hover.count} mention{hover.count === 1 ? '' : 's'}</div>
  </div>
{/if}
