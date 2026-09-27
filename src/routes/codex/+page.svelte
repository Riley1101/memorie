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
  import { resolve } from '$app/paths';
  import HomeIcon from '@lucide/svelte/icons/home';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import SearchIcon from '@lucide/svelte/icons/search';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
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
  import { codexManager, codexHref, KINDS, blankEntity } from '$lib/runes/codex.svelte.js';
  import { buildFileTree, formatFileName } from '$lib/utils.js';

  /** @typedef {import('$lib/runes/codex.svelte.js').Entity} Entity */

  /** '' is the shared codex on its own. Set from the URL in `afterNavigate`. */
  let binder = $state(appState.ui.activeBinder ?? '');
  let tab = $state('entries');
  let search = $state('');

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

  let groups = $derived(
    KINDS.map((kind) => ({ kind, items: filtered.filter((e) => e.kind === kind.id) }))
  );

  /** Writings (in binder order) that mention the open entry. */
  let appearances = $derived.by(() => {
    if (!matrix || !selected?.path || !binder) return null;
    const counts = Object.fromEntries(matrix.scenes.map((s) => [s.name, s.counts[selected.path] ?? 0]));
    const out = [];
    const walk = (nodes) => {
      for (const node of nodes) {
        if (node.type === 'folder') walk(node.children);
        else if (counts[node.file.name]) out.push({ name: node.file.name, label: formatFileName(node.name), count: counts[node.file.name] });
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
  async function open(entity) {
    if (entity.path === selectedPath && !creating) {
      tab = 'entries';
      return;
    }
    if (!(await leaveSheet())) return;
    renamed = null;
    creating = null;
    selectedPath = entity.path;
    tab = 'entries';
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
    tab = params.get('tab') === 'mentions' && binder ? 'mentions' : 'entries';
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
        ? entities.find((e) => e.name.toLowerCase() === lower || e.aliases.some((a) => a.toLowerCase() === lower))
        : null;
      if (existing) open(existing);
      else startNew(/** @type {any} */ (newKind), name);
    }
  });

  const kindIcons = {
    character: 'bg-primary/60',
    location: 'bg-teal/60',
    faction: 'bg-warning/60',
    item: 'bg-muted-foreground/50',
  };
</script>

{#snippet binderSelect(/** @type {string} */ width)}
  <select
    value={binder}
    onchange={(e) => switchBinder(e.currentTarget.value)}
    aria-label="Binder"
    class="h-8 {width} rounded-md border border-border bg-background px-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/50 shrink-0"
  >
    <option value="">Shared only</option>
    {#each fileManager.binders as b (b)}
      <option value={b}>{b}</option>
    {/each}
  </select>
{/snippet}

<div class="page-container w-full h-full flex flex-col overflow-hidden">
  <div class="flex items-center justify-between gap-4 mb-6 shrink-0">
    <h2 class="font-writer text-5xl font-normal text-heading-foreground">Codex</h2>
    <Button
      variant="ghost"
      size="icon"
      class="text-muted-foreground hover:text-foreground rounded-full transition-colors"
      onclick={() => goto(resolve('/'))}
      aria-label="Home"
      disabled={false}
    >
      <HomeIcon strokeWidth={1.5} class="size-5" />
    </Button>
  </div>

  <div class="flex items-center gap-1 mb-6 shrink-0 font-sans text-sm" role="tablist">
    {#each [{ id: 'entries', label: 'Sheets' }, { id: 'mentions', label: 'Mentions' }] as t (t.id)}
      <button
        type="button"
        role="tab"
        aria-selected={tab === t.id}
        disabled={t.id === 'mentions' && !binder}
        title={t.id === 'mentions' && !binder ? 'Pick a binder to see where entries are mentioned' : ''}
        onclick={async () => {
          if (t.id !== tab && !(await leaveSheet())) return;
          tab = t.id;
        }}
        class="px-3 py-1.5 rounded-md transition-colors disabled:opacity-40 disabled:pointer-events-none
          {tab === t.id ? 'bg-primary/10 text-primary font-medium' : 'text-muted-foreground hover:bg-muted/40 hover:text-foreground'}"
      >
        {t.label}
      </button>
    {/each}
  </div>

  {#if tab === 'mentions' && binder}
    <ScrollFade class="flex-1 min-h-0">
      <ScrollArea type="scroll" class="w-full h-full">
        <div class="pr-4 pb-8 flex flex-col gap-4">
          {@render binderSelect('w-60 max-w-full')}
          <CodexMatrix {binder} {matrix} loading={matrixLoading} onopen={open} />
        </div>
      </ScrollArea>
    </ScrollFade>
  {:else}
    <div class="flex-1 min-h-0 grid grid-cols-1 grid-rows-[minmax(0,1fr)] md:grid-cols-[15rem_1fr] gap-8">
      <!-- Narrow windows show the list or the sheet, not both. -->
      <aside class="min-h-0 flex-col gap-3 font-sans {selected ? 'hidden md:flex' : 'flex'}">
        {@render binderSelect('w-full')}
        <label class="flex items-center gap-2 h-8 px-2.5 rounded-lg border border-border bg-input/40 text-sm shrink-0">
          <SearchIcon strokeWidth={1.5} class="size-3.5 text-muted-foreground shrink-0" />
          <input bind:value={search} placeholder="Find an entry" class="flex-1 min-w-0 bg-transparent outline-none" />
        </label>
        <ScrollFade class="flex-1 min-h-0" fadeSize="h-4">
          <ScrollArea type="scroll" class="h-full">
            <nav class="flex flex-col gap-4 pb-4 pr-2">
              {#each groups as group (group.kind.id)}
                <div class="flex flex-col gap-0.5">
                  <div class="flex items-center justify-between px-2 pb-1">
                    <span class="text-[0.6875rem] font-medium text-muted-foreground/60">
                      {group.kind.plural}
                      <span class="tabular-nums opacity-70">{group.items.length || ''}</span>
                    </span>
                    <button
                      type="button"
                      onclick={() => startNew(group.kind.id)}
                      aria-label="New {group.kind.label.toLowerCase()}"
                      class="text-muted-foreground/50 hover:text-foreground"
                    >
                      <PlusIcon strokeWidth={1.5} class="size-3" />
                    </button>
                  </div>
                  {#each group.items as entity (entity.path)}
                    <button
                      type="button"
                      onclick={() => open(entity)}
                      class="flex items-center gap-2 px-2 py-1 rounded-md text-[0.8125rem] text-left transition-colors
                        {entity.path === selectedPath && !creating
                        ? 'bg-primary/10 text-primary font-medium'
                        : 'text-muted-foreground/90 hover:bg-muted/30 hover:text-foreground'}"
                    >
                      <span class="size-1.5 rounded-full shrink-0 {kindIcons[entity.kind]}"></span>
                      <span class="truncate flex-1">{entity.name}</span>
                      {#if !entity.scope && binder}
                        <span class="text-[0.625rem] text-muted-foreground/50" title="Shared by every binder">shared</span>
                      {/if}
                    </button>
                  {/each}
                </div>
              {/each}
            </nav>
          </ScrollArea>
        </ScrollFade>
      </aside>

      <ScrollFade class="min-h-0 {selected ? '' : 'hidden md:block'}">
        <ScrollArea type="scroll" class="h-full">
          <div class="pr-4 max-w-2xl">
            {#if selected}
              <button
                type="button"
                onclick={closeSheet}
                class="md:hidden mb-4 flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground font-sans"
              >
                <ChevronLeftIcon strokeWidth={1.5} class="size-4" /> All entries
              </button>
            {/if}
            {#if renamed && selected?.name === renamed.to}
              <div class="mb-6 flex items-center gap-3 rounded-lg border border-border/60 bg-muted/20 px-3 py-2 font-sans text-sm">
                <ReplaceIcon strokeWidth={1.5} class="size-4 shrink-0 text-muted-foreground" />
                <span class="flex-1 min-w-0">
                  Renamed from “{renamed.from}”. Your writings still say “{renamed.from}”.
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  onclick={() => {
                    if (!renamed) return;
                    appState.openProjectSearch({ query: renamed.from, replacement: renamed.to, binder: renamed.binder });
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
                  class="text-muted-foreground/60 hover:text-foreground"
                >
                  <XIcon strokeWidth={1.5} class="size-3.5" />
                </button>
              </div>
            {/if}
            {#if selected}
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
            {:else}
              <div class="flex flex-col items-center justify-center py-24 text-center gap-3 font-sans">
                <BookUserIcon strokeWidth={1.5} class="size-10 text-muted-foreground/30" />
                <p class="text-muted-foreground/70 max-w-sm">
                  {entities.length
                    ? 'Pick an entry, or start a new one.'
                    : `Keep sheets for the people, places, groups and things in ${binder || 'your stories'}. The assistant reads the ones a scene mentions.`}
                </p>
                <div class="flex flex-wrap justify-center gap-2 pt-2">
                  {#each KINDS as kind (kind.id)}
                    <Button variant="outline" size="sm" class="gap-1.5" onclick={() => startNew(kind.id)} disabled={false}>
                      <PlusIcon strokeWidth={1.5} class="size-3" />
                      {kind.label}
                    </Button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        </ScrollArea>
      </ScrollFade>
    </div>
  {/if}
</div>
