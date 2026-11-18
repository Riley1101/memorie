<script>
  /**
   * CodexMentions - where one codex entry appears in a binder: how often and
   * in how many writings, a bar per chapter, and every sentence that names it,
   * grouped by writing with the name marked. Choosing a sentence opens the
   * writing with that mention selected.
   */
  import { goto } from '$app/navigation';
  import { writingUrl } from '$lib/new-writing.js';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import CheckIcon from '@lucide/svelte/icons/check';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { codexManager } from '$lib/runes/codex.svelte.js';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { binderDot } from '$lib/binder-tints.js';
  import { buildFileTree, formatFileName } from '$lib/utils.js';

  /**
   * @typedef {import('$lib/runes/codex.svelte.js').Entity} Entity
   * @typedef {import('$lib/runes/codex.svelte.js').WritingMentions} WritingMentions
   */

  /** @type {{ binder: string, entity: Entity }} */
  let { binder, entity } = $props();

  /** @type {WritingMentions[] | null} */
  let found = $state(null);
  let failed = $state(false);
  let sort = $state(/** @type {'story' | 'most'} */ ('story'));

  $effect(() => {
    const target = { binder, path: entity.path };
    found = null;
    failed = false;
    codexManager
      .mentions(target.binder, target.path)
      .then((result) => {
        if (target.binder === binder && target.path === entity.path) found = result;
      })
      .catch((err) => {
        console.error('Could not read mentions:', err);
        failed = true;
      });
  });

  /** The binder's writings in story (tree) order, each with its chapter: the folder right under the binder. */
  let storyOrder = $derived.by(() => {
    /** @type {{ name: string, chapter: string }[]} */
    const out = [];
    const walk = (nodes, chapter) => {
      for (const node of nodes) {
        if (node.type === 'folder') walk(node.children, chapter ?? node.name);
        else out.push({ name: node.file.name, chapter: chapter ?? formatFileName(node.name) });
      }
    };
    walk(buildFileTree(fileManager.files, binder, fileManager.folders), null);
    return out;
  });

  let rank = $derived(new Map(storyOrder.map((w, i) => [w.name, i])));

  let writings = $derived.by(() => {
    if (!found) return [];
    const list = [...found];
    if (sort === 'most') list.sort((a, b) => b.count - a.count || (rank.get(a.name) ?? 0) - (rank.get(b.name) ?? 0));
    else list.sort((a, b) => (rank.get(a.name) ?? Infinity) - (rank.get(b.name) ?? Infinity));
    return list;
  });

  let total = $derived(found?.reduce((n, w) => n + w.count, 0) ?? 0);

  /** Mentions per chapter, in story order, for the bars. */
  let chapters = $derived.by(() => {
    const counts = new Map(found?.map((w) => [w.name, w.count]) ?? []);
    /** @type {{ label: string, count: number }[]} */
    const out = [];
    for (const w of storyOrder) {
      const last = out[out.length - 1];
      if (last?.label === w.chapter) last.count += counts.get(w.name) ?? 0;
      else out.push({ label: w.chapter, count: counts.get(w.name) ?? 0 });
    }
    return out.slice(0, 40);
  });
  let peak = $derived(Math.max(1, ...chapters.map((c) => c.count)));

  /** Which names matched, so "the keeper" turning up isn't a surprise. */
  let matched = $derived.by(() => {
    /** @type {string[]} */
    const seen = [];
    for (const w of found ?? []) for (const s of w.snippets) for (const p of s.parts) if (p.hit && !seen.includes(p.text)) seen.push(p.text);
    return seen.filter((t) => t !== entity.name);
  });

  /**
   * Opens the writing with this mention selected, the way find in all
   * writings does. Counted by the name's text, so a longer name that contains
   * it can shift which one is picked; the writing still opens.
   * @param {WritingMentions} writing
   * @param {number} snippetIndex
   * @param {string} text - The name as written.
   */
  function openMention(writing, snippetIndex, text) {
    const lower = text.toLowerCase();
    let occurrence = 0;
    for (const s of writing.snippets.slice(0, snippetIndex)) {
      for (const p of s.parts) if (p.hit && p.text.toLowerCase() === lower) occurrence += 1;
    }
    editorState.pendingReveal = {
      query: text,
      options: { caseSensitive: false, wholeWord: true, regex: false },
      occurrence,
    };
    goto(writingUrl(writing.name));
  }

  /** "Chapter 3" or the folder the writing sits in, beside its title. */
  function chapterOf(name) {
    return storyOrder.find((w) => w.name === name)?.chapter ?? '';
  }
</script>

<div class="max-w-160">
  <div class="flex flex-wrap items-end gap-5">
    <div class="flex-1 min-w-50">
      <h2 class="font-serif text-[28px] font-medium leading-tight">{entity.name}</h2>
      <p class="meta mt-0.5">
        {#if found}
          {total} mention{total === 1 ? '' : 's'} in {found.length} writing{found.length === 1 ? '' : 's'}{#if matched.length}
            · also matched as {matched.map((m) => `“${m}”`).join(', ')}{/if}
        {:else if failed}
          Couldn’t read the mentions.
        {:else}
          Counting mentions…
        {/if}
      </p>
    </div>
    {#if found && total > 0 && chapters.length > 1}
      <div class="w-45">
        <div
          class="flex items-end gap-1 h-9"
          role="img"
          aria-label="Mentions per chapter, in story order: {chapters.map((c) => `${c.label} ${c.count}`).join(', ')}"
        >
          {#each chapters as chapter, i (i)}
            <span
              title="{chapter.label}: {chapter.count}"
              class="flex-1 rounded-t-[2px] bg-primary/55"
              style="height: {Math.max(3, (chapter.count / peak) * 100)}%"
            ></span>
          {/each}
        </div>
        <div class="meta flex justify-between mt-1">
          <span class="truncate max-w-[40%]">{chapters[0].label}</span><span>by chapter</span><span class="truncate max-w-[40%]">{chapters[chapters.length - 1].label}</span>
        </div>
      </div>
    {/if}
  </div>

  {#if found && total > 0}
    <div class="flex flex-wrap items-center gap-1.5 mt-4.5 mb-4">
      <span class="chip is-on">All {total}</span>
      <span class="flex-1"></span>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <button {...props} type="button" class="chip">
              {sort === 'story' ? 'Story order' : 'Most mentions'}
              <ChevronDownIcon class="size-3" strokeWidth={2} />
            </button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content class="w-40" align="end" portalProps={{}}>
          {#each [{ id: 'story', label: 'Story order' }, { id: 'most', label: 'Most mentions' }] as option (option.id)}
            <DropdownMenu.Item class="" inset={false} onclick={() => (sort = /** @type {'story' | 'most'} */ (option.id))}>
              <span class="flex-1">{option.label}</span>
              {#if sort === option.id}<CheckIcon class="size-3.5" />{/if}
            </DropdownMenu.Item>
          {/each}
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </div>
  {/if}

  <div class="flex flex-col gap-3 mt-4">
    {#if !found && !failed}
      {#each [0, 1] as i (i)}
        <div class="inset-group p-3.5 flex flex-col gap-2.5">
          <Skeleton class="h-3.5 w-40 rounded" />
          <Skeleton class="h-4 w-full rounded" />
          <Skeleton class="h-4 w-4/5 rounded" />
        </div>
      {/each}
    {:else if found && total === 0}
      <p class="text-sm text-muted-foreground py-8">
        No writing in {binder} names {entity.name}{entity.aliases.length ? ' or its aliases' : ''} yet.
        {#if entity.matchMode === 'off'}Mention detection is off for this entry.{/if}
      </p>
    {/if}
    {#each writings as writing (writing.name)}
      {@const chapter = chapterOf(writing.name)}
      {@const title = formatFileName(writing.name.split('/').pop() ?? '')}
      <section class="inset-group">
        <div class="flex items-center gap-2 px-3.5 pt-3 pb-2">
          <span class="binder-dot {binderDot(binder)}"></span>
          <h3 class="text-[13.5px] font-semibold truncate">{title}</h3>
          {#if chapter && chapter !== title}<span class="meta truncate">{chapter}</span>{/if}
          <span class="meta ml-auto shrink-0">{writing.count} mention{writing.count === 1 ? '' : 's'}</span>
        </div>
        {#each writing.snippets as snippet, i (i)}
          {@const first = snippet.parts.find((p) => p.hit)?.text ?? entity.name}
          <button
            type="button"
            onclick={() => openMention(writing, i, first)}
            class="grid w-full grid-cols-[minmax(0,1fr)_auto] items-center gap-3.5 px-3.5 py-2.5 text-left shadow-[0_-1px_0_var(--hair)] hover:bg-fill transition-colors"
          >
            <span class="font-serif text-[14.5px] leading-normal">
              {#each snippet.parts as part, j (j)}{#if part.hit}<mark class="rounded-[3px] bg-selection px-0.5 text-inherit">{part.text}</mark>{:else}{part.text}{/if}{/each}
            </span>
            <span class="meta whitespace-nowrap">¶ {snippet.paragraph}</span>
          </button>
        {/each}
      </section>
    {/each}
  </div>
</div>
