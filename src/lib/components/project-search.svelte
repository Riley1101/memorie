<script>
  import { invoke } from '@tauri-apps/api/core';
  import { goto, invalidateAll } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager, baseOf, dirOf } from '$lib/runes/fs.svelte.js';
  import { formatFileName } from '$lib/utils.js';
  import { toast } from '$lib/toast.js';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ReplaceIcon from '@lucide/svelte/icons/replace';

  /**
   * @typedef {{ line: number, preview: string, start: number, end: number, meta: boolean }} Match
   * @typedef {{ name: string, matches: Match[] }} FileMatches
   * @typedef {{ files: FileMatches[], total: number, truncated: boolean }} Results
   */

  const SEARCH_DEBOUNCE_MS = 200;

  let query = $state('');
  let replacement = $state('');
  let showReplace = $state(false);
  let caseSensitive = $state(false);
  let wholeWord = $state(false);
  let useRegex = $state(false);

  /** @type {Results | null} */
  let results = $state(null);
  let error = $state('');
  let searching = $state(false);
  let replacing = $state(false);
  /** Files whose match list is collapsed. @type {Record<string, true>} */
  let collapsed = $state({});
  /** Asks for a second click before replacing everywhere. */
  let confirmAll = $state(false);

  let inputEl = $state(/** @type {HTMLInputElement | null} */ (null));

  let options = $derived({ caseSensitive, wholeWord, regex: useRegex });

  /** Bumped per search so a slow earlier search can't overwrite a newer one. */
  let generation = 0;

  async function runSearch() {
    const current = ++generation;
    const q = query;
    if (!q) {
      results = null;
      error = '';
      return;
    }
    searching = true;
    try {
      /** @type {Results} */
      const found = await invoke('search_project', { query: q, options });
      if (current !== generation) return;
      results = found;
      error = '';
    } catch (e) {
      if (current !== generation) return;
      results = null;
      error = typeof e === 'string' ? e : String(e);
    } finally {
      if (current === generation) searching = false;
    }
  }

  // Re-search while typing or toggling options.
  $effect(() => {
    void query;
    void options;
    if (!appState.ui.isProjectSearchOpen) return;
    confirmAll = false;
    const timer = setTimeout(runSearch, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (appState.ui.isProjectSearchOpen) {
      // The dialog focuses its first element; select any previous query instead.
      queueMicrotask(() => inputEl?.select());
    }
  });

  /** Set when a result was chosen, so closing doesn't pull focus back to the dialog's trigger. */
  let keepFocusOnClose = false;

  /**
   * Opens a writing and selects the match. Matches in the front matter
   * (synopsis, status) open the scene panel instead, since the editor
   * doesn't show that part.
   * @param {FileMatches} file
   * @param {number} index
   */
  function openMatch(file, index) {
    const match = file.matches[index];
    if (match.meta) {
      editorState.pendingReveal = null;
      appState.toggleOutline(true);
    } else {
      // The editor only sees the text, so count only matches outside the front matter.
      const occurrence = file.matches.slice(0, index).filter((m) => !m.meta).length;
      editorState.pendingReveal = { query, options: { ...options }, occurrence };
    }
    const name = file.name;
    keepFocusOnClose = true;
    appState.toggleProjectSearch(false);
    const url = resolve(`/${encodeURIComponent(name)}`);
    if (page.params.lexical === name) {
      editorState.revealPending?.();
    } else {
      goto(url);
    }
  }

  /** @param {string[]} names */
  async function replaceIn(names) {
    if (!query || names.length === 0) return;
    replacing = true;
    try {
      // Unsaved typing in the open writing would otherwise overwrite the replace.
      if (editorState.flushSave) await editorState.flushSave();
      /** @type {{ name: string, count: number }[]} */
      const replaced = await invoke('replace_in_project', {
        query,
        replacement,
        options,
        names,
      });
      const total = replaced.reduce((sum, r) => sum + r.count, 0);
      if (total === 0) {
        toast.info('Nothing replaced');
      } else {
        toast.success(
          `Replaced ${total} ${total === 1 ? 'match' : 'matches'} in ${replaced.length} ${replaced.length === 1 ? 'writing' : 'writings'}`,
          'Each change is in the writing’s version history.'
        );
      }
      const open = page.params.lexical;
      if (open && replaced.some((r) => r.name === open)) {
        await invalidateAll();
        appState.incrementEditorVersion();
      }
      await fileManager.getRecents();
      await runSearch();
    } catch (e) {
      toast.error('Replace failed', e);
    } finally {
      replacing = false;
      confirmAll = false;
    }
  }

  function replaceAll() {
    if (!confirmAll) {
      confirmAll = true;
      return;
    }
    replaceIn(results?.files.map((f) => f.name) ?? []);
  }

  /** @param {string} name */
  function toggleFile(name) {
    const next = { ...collapsed };
    if (next[name]) delete next[name];
    else next[name] = true;
    collapsed = next;
  }

  /** @param {Match} m */
  function parts(m) {
    return [m.preview.slice(0, m.start), m.preview.slice(m.start, m.end), m.preview.slice(m.end)];
  }

  let fileCount = $derived(results?.files.length ?? 0);

  let summary = $derived.by(() => {
    if (error) return error;
    if (!query) return 'Search every writing in your library.';
    if (!results) return searching ? 'Searching…' : '';
    if (results.total === 0) return 'No matches.';
    const matches = `${results.total}${results.truncated ? '+' : ''} ${results.total === 1 ? 'match' : 'matches'}`;
    return `${matches} in ${fileCount} ${fileCount === 1 ? 'writing' : 'writings'}`;
  });
</script>

{#snippet toggle(
  /** @type {string} */ label,
  /** @type {string} */ title,
  /** @type {boolean} */ pressed,
  /** @type {() => void} */ onclick
)}
  <button
    type="button"
    {title}
    aria-label={title}
    aria-pressed={pressed}
    {onclick}
    class="h-7 min-w-7 px-1.5 rounded-md font-mono text-xs transition-colors
      {pressed ? 'bg-primary/15 text-primary' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
  >
    {label}
  </button>
{/snippet}

<Dialog.Root
  open={appState.ui.isProjectSearchOpen}
  onOpenChange={(v) => appState.toggleProjectSearch(v)}
>
  <Dialog.Content
    class="sm:max-w-2xl p-0 gap-0 overflow-hidden font-sans {appState.ui.theme}"
    portalProps={{}}
    onCloseAutoFocus={(e) => {
      if (keepFocusOnClose) e.preventDefault();
      keepFocusOnClose = false;
    }}
  >
    <Dialog.Title class="sr-only">Find in all writings</Dialog.Title>
    <Dialog.Description class="sr-only">Search and replace across every writing</Dialog.Description>

    <div class="grid gap-2 p-3 pr-12 border-b border-border/50">
      <div class="flex items-center gap-1">
        <button
          type="button"
          onclick={() => (showReplace = !showReplace)}
          aria-label={showReplace ? 'Hide replace' : 'Show replace'}
          aria-expanded={showReplace}
          class="size-7 flex items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          <ChevronRightIcon
            strokeWidth={1.5}
            class="size-4 transition-transform {showReplace ? 'rotate-90' : ''}"
          />
        </button>
        <input
          bind:this={inputEl}
          bind:value={query}
          placeholder="Find in all writings"
          aria-label="Find"
          spellcheck="false"
          autocomplete="off"
          class="flex-1 h-8 rounded-md border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50
            {error ? 'border-destructive' : 'border-border'}"
        />
        {@render toggle('Aa', 'Match case', caseSensitive, () => (caseSensitive = !caseSensitive))}
        {@render toggle('ab', 'Whole word', wholeWord, () => (wholeWord = !wholeWord))}
        {@render toggle('.*', 'Regular expression', useRegex, () => (useRegex = !useRegex))}
      </div>

      {#if showReplace}
        <div class="flex items-center gap-1 pl-8">
          <input
            bind:value={replacement}
            placeholder={useRegex ? 'Replace ($1 for groups)' : 'Replace'}
            aria-label="Replace"
            spellcheck="false"
            autocomplete="off"
            class="flex-1 h-8 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          />
          <Button
            size="sm"
            variant={confirmAll ? 'destructive' : 'secondary'}
            disabled={replacing || !results || results.total === 0}
            onclick={replaceAll}
          >
            <ReplaceIcon strokeWidth={1.5} />
            {confirmAll ? `Replace ${results?.total ?? 0}?` : 'Replace all'}
          </Button>
        </div>
      {/if}

      <p class="pl-8 text-xs {error ? 'text-destructive' : 'text-muted-foreground'}" aria-live="polite">
        {summary}
      </p>
    </div>

    <div class="max-h-[60vh] overflow-y-auto p-2">
      {#each results?.files ?? [] as file (file.name)}
        {@const isOpen = !collapsed[file.name]}
        <section class="mb-1">
          <div class="group flex items-center gap-1 rounded-md hover:bg-muted/40">
            <button
              type="button"
              onclick={() => toggleFile(file.name)}
              aria-expanded={isOpen}
              class="flex flex-1 items-center gap-1.5 min-w-0 px-1.5 py-1.5 text-left"
            >
              <ChevronRightIcon
                strokeWidth={1.5}
                class="size-3.5 shrink-0 text-muted-foreground transition-transform {isOpen ? 'rotate-90' : ''}"
              />
              <span class="truncate text-sm font-medium">{formatFileName(baseOf(file.name))}</span>
              {#if dirOf(file.name)}
                <span class="truncate text-xs text-muted-foreground/60">{dirOf(file.name)}</span>
              {/if}
              <span class="ml-auto shrink-0 rounded-full bg-muted px-1.5 text-[0.6875rem] tabular-nums text-muted-foreground">
                {file.matches.length}
              </span>
            </button>
            {#if showReplace}
              <Button
                size="xs"
                variant="ghost"
                class="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 mr-1"
                disabled={replacing}
                onclick={() => replaceIn([file.name])}
              >
                Replace
              </Button>
            {/if}
          </div>
          {#if isOpen}
            <ul>
              {#each file.matches as match, i (i)}
                {@const [before, hit, after] = parts(match)}
                <li>
                  <button
                    type="button"
                    onclick={() => openMatch(file, i)}
                    class="w-full flex items-baseline gap-2 rounded-md pl-7 pr-2 py-1 text-left hover:bg-muted/60 focus-visible:bg-muted/60 outline-none"
                  >
                    <span class="w-8 shrink-0 text-right text-[0.6875rem] tabular-nums text-muted-foreground/50">
                      {match.meta ? 'info' : match.line}
                    </span>
                    <span class="min-w-0 truncate font-writer text-sm text-muted-foreground">
                      {before}<mark class="rounded-sm bg-warning/40 text-foreground px-0.5"
                        >{hit}</mark
                      >{#if showReplace && replacement !== undefined && query}<ins
                          class="no-underline rounded-sm bg-success/25 text-foreground px-0.5"
                          >{replacement}</ins
                        >{/if}{after}
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/each}
    </div>
  </Dialog.Content>
</Dialog.Root>
