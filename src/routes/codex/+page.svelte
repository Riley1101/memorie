<script>
  /**
   * The codex of a binder: character, location, faction and item sheets on one
   * tab, and which writings mention them on the other.
   *
   * Query: `binder` (omit for the shared codex), `entry` (a sheet to open),
   * `tab` (`mentions` for the matrix), `new` (a kind, with `name`, to start a sheet).
   */
  import { page } from '$app/state';
  import { goto, replaceState, beforeNavigate, afterNavigate } from '$app/navigation';
  import PanelLeftIcon from '@lucide/svelte/icons/panel-left';
  import BinderSidebar from '$lib/components/binder-sidebar.svelte';
  import CodexMentions from '$lib/components/codex-mentions.svelte';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { binderDot } from '$lib/binder-tints.js';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import SearchIcon from '@lucide/svelte/icons/search';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import { Button } from '$lib/components/ui/button/index.js';
  import { ScrollArea } from '@/components/ui/scroll-area/index.js';
  import ScrollFade from '$lib/components/scroll-fade.svelte';
  import CodexSheet from '$lib/components/codex-sheet.svelte';
  import CodexMatrix from '$lib/components/codex-matrix.svelte';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import ReplaceIcon from '@lucide/svelte/icons/replace';
  import XIcon from '@lucide/svelte/icons/x';
  import {
    codexManager,
    codexHref,
    KINDS,
    kindInfo,
    blankEntity,
  } from '$lib/runes/codex.svelte.js';
  import { buildFileTree, formatFileName } from '$lib/utils.js';

  /** @typedef {import('$lib/runes/codex.svelte.js').Entity} Entity */

  /**
   * A kind's colour as a dot, so an entry row reads like a writing row: one
   * marker, the name, one number. Written out because Tailwind needs the class.
   * @type {Record<string, string>}
   */
  const KIND_DOT = {
    character: 'bg-binder-clay',
    location: 'bg-binder-teal',
    faction: 'bg-binder-plum',
    item: 'bg-binder-ochre',
  };

  /** '' is the shared codex on its own. Set from the URL in `afterNavigate`. */
  let binder = $state(appState.ui.activeBinder ?? '');
  let tab = $state('entries');
  let search = $state('');
  /** The kind shown on the shelf, '' for all of them. */
  let kindFilter = $state('');

  let selectedPath = $state('');
  /** A sheet not saved yet. @type {Entity | null} */
  let creating = $state(null);

  /** @type {{ save: () => Promise<boolean>, hasUnsavedChanges: () => boolean } | null} */
  let sheet = $state(null);

  /** The last rename, so the writer can carry it into their writings. @type {{ from: string, to: string, binder: string | null } | null} */
  let renamed = $state(null);

  /** @type {import('$lib/runes/codex.svelte.js').Matrix | null} */
  let matrix = $state(null);
  let matrixLoading = $state(false);

  let entities = $derived(codexManager.entities(binder || null));
  let selected = $derived(creating ?? entities.find((e) => e.path === selectedPath) ?? null);

  let filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return entities;
    return entities.filter(
      (e) =>
        e.name.toLowerCase().includes(q) ||
        e.aliases.some((a) => a.toLowerCase().includes(q)) ||
        e.summary.toLowerCase().includes(q)
    );
  });

  /** What the shelf shows: the search, narrowed to one kind if asked. */
  let visible = $derived(
    (kindFilter ? filtered.filter((e) => e.kind === kindFilter) : filtered)
      .slice()
      .sort((a, b) => a.name.localeCompare(b.name))
  );

  /** The shelf the open sheet came from, so prev/next walk what was browsed. */
  let neighbours = $derived(visible.length ? visible : entities);
  let entryIndex = $derived(neighbours.findIndex((e) => e.path === selectedPath));
  let prevEntry = $derived(entryIndex > 0 ? neighbours[entryIndex - 1] : null);
  let nextEntry = $derived(
    entryIndex >= 0 && entryIndex < neighbours.length - 1 ? neighbours[entryIndex + 1] : null
  );

  /** The first filled field of a sheet, for a card with no summary. @param {Entity} entity */
  function firstField(entity) {
    return entity.fields.find((f) => f.value.trim())?.value.trim() ?? '';
  }

  /** Writings (in binder order) that mention the open entry. */
  let appearances = $derived.by(() => {
    if (!matrix || !selected?.path || !binder) return null;
    const counts = Object.fromEntries(
      matrix.scenes.map((s) => [s.name, s.counts[selected.path] ?? 0])
    );
    const out = [];
    const walk = (nodes) => {
      for (const node of nodes) {
        if (node.type === 'folder') walk(node.children);
        else if (counts[node.file.name])
          out.push({
            name: node.file.name,
            label: formatFileName(node.name),
            count: counts[node.file.name],
          });
      }
    };
    walk(buildFileTree(fileManager.files, binder, fileManager.folders));
    return out;
  });

  async function loadMatrix() {
    if (!binder) {
      matrix = null;
      return;
    }
    const target = binder;
    matrixLoading = true;
    try {
      const result = await codexManager.matrix(target);
      if (binder === target) matrix = result;
    } catch (err) {
      console.error('Could not build the mentions matrix:', err);
    } finally {
      matrixLoading = false;
    }
  }

  $effect(() => {
    const b = binder;
    codexManager.load(b || null);
    matrix = null;
    loadMatrix();
  });

  /** Keeps the URL in step, so going back returns to the same sheet. */
  $effect(() => {
    const next = codexHref({
      binder,
      tab: tab === 'mentions' ? 'mentions' : '',
      entry: creating ? '' : selectedPath,
    });
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- codexHref resolves the path
    if (next !== `${page.url.pathname}${page.url.search}`) replaceState(next, {});
  });

  /**
   * Saves the open sheet before leaving it.
   * @returns {Promise<boolean>} false if it couldn't be saved, so the writer stays on it.
   */
  async function leaveSheet() {
    return sheet ? await sheet.save() : true;
  }

  /** @param {Entity} entity */
  /** Opens an entry in the current tab: its sheet, or where it is mentioned. */
  async function open(entity) {
    if (entity.path === selectedPath && !creating) return;
    if (!(await leaveSheet())) return;
    renamed = null;
    creating = null;
    selectedPath = entity.path;
  }

  /** @param {import('$lib/runes/codex.svelte.js').EntityKind} kind @param {string} [name] */
  async function startNew(kind, name = '') {
    if (!(await leaveSheet())) return;
    renamed = null;
    selectedPath = '';
    creating = blankEntity(kind, binder || null, name);
    tab = 'entries';
  }

  /** @param {Entity} saved @param {Entity} previous */
  function onsaved(saved, previous) {
    renamed =
      previous.path && previous.name !== saved.name
        ? { from: previous.name, to: saved.name, binder: saved.scope }
        : null;
    creating = null;
    if ((saved.scope ?? '') !== binder && saved.scope) binder = saved.scope;
    selectedPath = saved.path;
    loadMatrix();
  }

  function ondeleted() {
    selectedPath = '';
    creating = null;
    loadMatrix();
  }

  /** @param {string} next */
  async function switchBinder(next) {
    if (!(await leaveSheet())) return;
    creating = null;
    selectedPath = '';
    binder = next;
  }

  /** Back to the list on narrow windows, where the list and sheet take turns. */
  async function closeSheet() {
    if (!(await leaveSheet())) return;
    creating = null;
    selectedPath = '';
  }

  /** Saves the open sheet on the way out of the page; a failed save keeps the writer here. */
  let leaving = false;
  beforeNavigate((nav) => {
    if (leaving || nav.to?.route.id === '/codex' || !sheet?.hasUnsavedChanges()) return;
    if (nav.type === 'leave') return; // Closing the window: nothing to wait on.
    nav.cancel();
    const target = nav.to?.url;
    sheet.save().then((ok) => {
      if (!ok || !target) return;
      leaving = true;
      // eslint-disable-next-line svelte/no-navigation-without-resolve -- replaying a navigation SvelteKit already resolved
      goto(target).finally(() => (leaving = false));
    });
  });

  /**
   * Reads the URL: on arrival, and when a link opens the codex while it's already
   * open (the page stays mounted, so this is the only place that sees the new query).
   */
  afterNavigate(async (nav) => {
    const params = page.url.searchParams;
    const arriving = nav.from?.route.id !== '/codex';
    if (!arriving && !(await leaveSheet())) return;
    // Arriving without a binder means the one picked on the home screen.
    binder = params.get('binder') ?? (arriving ? (appState.ui.activeBinder ?? '') : '');
    tab = params.get('tab') === 'mentions' ? 'mentions' : 'entries';
    selectedPath = params.get('entry') ?? '';
    creating = null;
    renamed = null;

    if (!fileManager.binders.length) await fileManager.getBinders();
    if (!fileManager.hasLoadedFiles) await fileManager.getRecents();
    if (!fileManager.folders.length) await fileManager.getFolders();
    const newKind = params.get('new');
    if (newKind && KINDS.some((k) => k.id === newKind)) {
      const name = (params.get('name') ?? '').trim();
      // "Add to codex" on a name that already has a sheet opens that sheet.
      if (name) await codexManager.load(binder || null);
      const lower = name.toLowerCase();
      const existing = name
        ? entities.find(
            (e) =>
              e.name.toLowerCase() === lower || e.aliases.some((a) => a.toLowerCase() === lower)
          )
        : null;
      if (existing) open(existing);
      else startNew(/** @type {any} */ (newKind), name);
    }
  });

  /** How often each entry is mentioned across the binder (C5: shown in the entry list). */
  let mentionCounts = $derived.by(() => {
    /** @type {Record<string, number>} */
    const totals = {};
    for (const scene of matrix?.scenes ?? []) {
      for (const [path, n] of Object.entries(scene.counts)) totals[path] = (totals[path] ?? 0) + n;
    }
    return totals;
  });

  let sidebarOpen = $state(false);

  /** @param {string} id */
  async function switchTab(id) {
    if (id !== tab && !(await leaveSheet())) return;
    tab = id;
  }
</script>

{#snippet binderSelect()}
  <!-- A chip in the toolbar; the native control stays for its keyboard and
       screen reader behaviour. -->
  <span class="relative shrink-0 min-w-0 max-w-56">
    <span
      aria-hidden="true"
      class="binder-dot pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 {binder
        ? binderDot(binder)
        : 'bg-muted-foreground/40'}"
    ></span>
    <select
      value={binder}
      onchange={(e) => switchBinder(e.currentTarget.value)}
      aria-label="Binder"
      class="appearance-none w-full h-6.5 rounded-full bg-fill pl-7 pr-7 text-xs text-foreground truncate hover:bg-[color-mix(in_oklch,var(--foreground)_9%,transparent)] transition-colors"
    >
      <option value="">Shared only</option>
      {#each fileManager.binders as b (b)}
        <option value={b}>{b}</option>
      {/each}
    </select>
    <ChevronDownIcon
      strokeWidth={2}
      class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 size-3 text-muted-foreground"
    />
  </span>
{/snippet}

{#snippet emptySheet()}
  <div class="flex flex-col items-center justify-center py-24 text-center gap-2">
    <BookUserIcon strokeWidth={1.4} class="size-8 text-muted-foreground/40 mb-1" />
    <p class="text-sm text-muted-foreground max-w-sm">
      {entities.length
        ? 'Pick an entry, or start a new one.'
        : `Keep sheets for the people, places, groups and things in ${binder || 'your stories'}. The assistant reads the ones a scene mentions.`}
    </p>
    <div class="flex flex-wrap justify-center gap-2 pt-2">
      <!-- One clear first step (most codexes start with people), the rest secondary. -->
      {#each KINDS as kind, i (kind.id)}
        <Button
          variant={i === 0 ? 'default' : 'ghost'}
          size="sm"
          class="gap-1.5"
          onclick={() => startNew(kind.id)}
          disabled={false}
        >
          <PlusIcon strokeWidth={1.8} class="size-3" />
          New {kind.label.toLowerCase()}
        </Button>
      {/each}
    </div>
  </div>
{/snippet}

<div class="relative flex w-full h-full overflow-hidden gap-3 p-3">
  <BinderSidebar bind:activeBinder={appState.ui.activeBinder} bind:sheetOpen={sidebarOpen} />

  <main class="lifted flex-1 min-w-0 h-full flex flex-col rounded-group overflow-hidden">
    <header
      data-tauri-drag-region="deep"
      class="toolbar max-lg:pl-[max(20px,var(--titlebar-inset-left,0px))]"
    >
      <button
        type="button"
        class="tb-btn lg:hidden -ml-2"
        aria-label="Show library sidebar"
        onclick={() => (sidebarOpen = true)}
      >
        <PanelLeftIcon strokeWidth={1.7} />
      </button>
      <h1 class="toolbar-title shrink-0">Codex</h1>
      {@render binderSelect()}
      <div class="capsule glass ml-auto" role="tablist" aria-label="Codex view">
        {#each [{ id: 'entries', label: 'Sheets' }, { id: 'mentions', label: 'Mentions' }] as t (t.id)}
          <button
            type="button"
            role="tab"
            class="tb-btn px-2.5"
            aria-selected={tab === t.id}
            onclick={() => switchTab(t.id)}
          >
            {t.label}
          </button>
        {/each}
      </div>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <button {...props} type="button" class="capsule tb-primary p-0">
              <span class="tb-btn px-3">
                <PlusIcon strokeWidth={2} class="size-[15px]!" />
                <span class="hidden sm:inline">New entry</span>
              </span>
            </button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content class="w-44" align="end" portalProps={{}}>
          {#each KINDS as kind (kind.id)}
            <DropdownMenu.Item class="" inset={false} onclick={() => startNew(kind.id)}
              >New {kind.label.toLowerCase()}</DropdownMenu.Item
            >
          {/each}
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </header>

    <!-- Browse, then open: the codex reads like the library. A grid of entry
         cards on the paper, and opening one gives the sheet the full width,
         with the way back and the neighbours in its header. -->
    <ScrollFade class="flex-1 min-h-0" fade={20}>
      <ScrollArea type="scroll" class="h-full">
        {#if selected}
          <div class="flex items-center gap-1 px-6 pt-4">
            <button
              type="button"
              onclick={closeSheet}
              class="flex items-center gap-1 h-7 -ml-2 px-2 rounded-full text-sm text-muted-foreground hover:bg-fill hover:text-foreground transition-colors"
            >
              <ChevronLeftIcon strokeWidth={1.7} class="size-4" />
              All entries
            </button>
            {#if !creating && neighbours.length > 1}
              <!-- The writing editor's own pattern: step to the next sheet
                   without going back to the grid first. -->
              <span class="ml-auto flex items-center gap-1">
                <button
                  type="button"
                  onclick={() => prevEntry && open(prevEntry)}
                  disabled={!prevEntry}
                  aria-label="Previous entry"
                  title={prevEntry ? `Previous: ${prevEntry.name}` : 'Previous entry'}
                  class="flex items-center justify-center size-7 rounded-full text-muted-foreground hover:bg-fill hover:text-foreground disabled:opacity-35 disabled:pointer-events-none transition-colors"
                >
                  <ChevronLeftIcon strokeWidth={1.7} class="size-4" />
                </button>
                <span class="meta tabular-nums">{entryIndex + 1} / {neighbours.length}</span>
                <button
                  type="button"
                  onclick={() => nextEntry && open(nextEntry)}
                  disabled={!nextEntry}
                  aria-label="Next entry"
                  title={nextEntry ? `Next: ${nextEntry.name}` : 'Next entry'}
                  class="flex items-center justify-center size-7 rounded-full text-muted-foreground hover:bg-fill hover:text-foreground disabled:opacity-35 disabled:pointer-events-none transition-colors"
                >
                  <ChevronRightIcon strokeWidth={1.7} class="size-4" />
                </button>
              </span>
            {/if}
          </div>
        {/if}

        <article class="px-6 pt-4 pb-14">
          {#if tab === 'mentions' && !selected}
            {#if !binder}
              <!-- Mentions are counted per binder; say so here instead of disabling the tab. -->
              <div class="flex flex-col gap-1 py-10">
                <p class="text-title font-semibold">Pick a binder to see mentions</p>
                <p class="text-sm text-muted-foreground max-w-sm">
                  Mentions show where each entry is named in a binder's writings. Choose a binder in
                  the toolbar.
                </p>
              </div>
            {:else}
              <div class="max-w-4xl">
                <CodexMatrix {binder} {matrix} loading={matrixLoading} onopen={open} />
              </div>
            {/if}
          {:else if selected}
            {#if tab === 'mentions' && !creating}
              {#key selected.path}
                <CodexMentions {binder} entity={selected} />
              {/key}
            {:else}
              <div class="max-w-2xl">
                {#if renamed && selected?.name === renamed.to}
                  <div class="mb-6 flex items-center gap-3 rounded-group bg-fill px-3 py-2 text-sm">
                    <ReplaceIcon strokeWidth={1.6} class="size-4 shrink-0 text-muted-foreground" />
                    <span class="flex-1 min-w-0">
                      Renamed from "{renamed.from}". Your writings still say "{renamed.from}".
                    </span>
                    <Button
                      size="sm"
                      variant="outline"
                      onclick={() => {
                        if (!renamed) return;
                        appState.openProjectSearch({
                          query: renamed.from,
                          replacement: renamed.to,
                          binder: renamed.binder,
                        });
                        renamed = null;
                      }}
                      disabled={false}
                      class=""
                    >
                      Find and replace…
                    </Button>
                    <button
                      type="button"
                      onclick={() => (renamed = null)}
                      aria-label="Dismiss"
                      class="text-muted-foreground hover:text-foreground"
                    >
                      <XIcon strokeWidth={1.6} class="size-3.5" />
                    </button>
                  </div>
                {/if}
                {#key creating ? `new:${creating.kind}` : selected.path}
                  <CodexSheet
                    bind:this={sheet}
                    entity={selected}
                    binders={fileManager.binders}
                    names={entities.filter((e) => e.path !== selected.path).map((e) => e.name)}
                    {appearances}
                    {onsaved}
                    {ondeleted}
                  />
                {/key}
              </div>
            {/if}
          {:else if entities.length}
            <!-- The shelf: one card per entry, in the grid the library uses. -->
            <div class="flex flex-wrap items-center gap-2 pb-5">
              <button
                type="button"
                class="chip"
                aria-pressed={kindFilter === ''}
                onclick={() => (kindFilter = '')}
              >
                All <span class="tabular-nums opacity-70">{entities.length}</span>
              </button>
              {#each KINDS as kind (kind.id)}
                {@const count = entities.filter((e) => e.kind === kind.id).length}
                <button
                  type="button"
                  class="chip"
                  aria-pressed={kindFilter === kind.id}
                  onclick={() => (kindFilter = kind.id)}
                >
                  <span class="size-2 rounded-full {KIND_DOT[kind.id]}"></span>
                  {kind.plural}
                  <span class="tabular-nums opacity-70">{count}</span>
                </button>
              {/each}
              <label class="field ml-auto w-48 max-sm:w-full">
                <SearchIcon strokeWidth={1.8} class="size-3.5 shrink-0" />
                <input bind:value={search} placeholder="Find an entry" aria-label="Find an entry" />
              </label>
            </div>

            {#if visible.length}
              <div class="wgrid">
                {#each visible as entity (entity.path)}
                  <div class="wcard">
                    <button
                      type="button"
                      onclick={() => open(entity)}
                      class="wcard-page"
                      aria-label="Open {entity.name}"
                    >
                      <p class="text-xs leading-relaxed text-muted-foreground line-clamp-6">
                        {#if entity.summary}
                          {entity.summary}
                        {:else if firstField(entity)}
                          {firstField(entity)}
                        {:else}
                          <span class="italic">No summary yet</span>
                        {/if}
                      </p>
                      <span class="wcard-fade"></span>
                    </button>
                    <div class="wcard-body">
                      <div class="wcard-title">{entity.name}</div>
                      <div class="flex items-center gap-1.5 meta">
                        <span
                          class="size-2 rounded-full shrink-0 {KIND_DOT[entity.kind] ??
                            KIND_DOT.character}"
                        ></span>
                        <span class="truncate">{kindInfo(entity.kind).label}</span>
                        {#if !entity.scope && binder}
                          <span title="Shared by every binder">· shared</span>
                        {/if}
                        {#if mentionCounts[entity.path]}
                          <span class="ml-auto shrink-0" title="Mentions in {binder}"
                            >{mentionCounts[entity.path]}</span
                          >
                        {/if}
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="py-16 text-center text-sm text-muted-foreground">No entry matches that.</p>
            {/if}
          {:else}
            {@render emptySheet()}
          {/if}
        </article>
      </ScrollArea>
    </ScrollFade>
  </main>
</div>
