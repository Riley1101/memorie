<script>
  import { editorViewCtx } from '@milkdown/kit/core';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { isMod, MOD_KEY } from '$lib/keyboard.svelte.js';
  import { toast } from '$lib/toast.js';
  import {
    buildFindRegex,
    setFind,
    findStep,
    replaceCurrent,
    replaceAll,
    watchFind,
  } from '$lib/components/plugins/find.js';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
  import TextSearchIcon from '@lucide/svelte/icons/text-search';
  import XIcon from '@lucide/svelte/icons/x';

  /**
   * @file Find and replace bar for the open writing (⌘F). Highlighting and
   * replacing live in the `find` editor plugin; this is the input.
   */

  const SEARCH_DEBOUNCE_MS = 120;

  let open = $state(false);
  let query = $state('');
  let replacement = $state('');
  let showReplace = $state(false);
  let caseSensitive = $state(false);
  let wholeWord = $state(false);
  let useRegex = $state(false);
  let error = $state('');
  /** Mirrors the plugin's match count and current index for the counter. */
  let total = $state(0);
  let current = $state(-1);

  let inputEl = $state(/** @type {HTMLInputElement | null} */ (null));

  let options = $derived({ caseSensitive, wholeWord, regex: useRegex });

  $effect(() =>
    watchFind((find) => {
      total = find.matches.length;
      current = find.current;
    })
  );

  /** @param {(view: import('@milkdown/kit/prose/view').EditorView) => void} fn */
  function withView(fn) {
    editorState.editor?.action((ctx) => fn(ctx.get(editorViewCtx)));
  }

  /** A search is waiting on the debounce. */
  let pending = false;

  function runSearch() {
    pending = false;
    let regex = null;
    error = '';
    if (open && query) {
      try {
        regex = buildFindRegex(query, options);
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
    }
    withView((view) => setFind(view, regex, useRegex));
  }

  // Re-search while typing or toggling options, and again when the editor
  // remounts under an open bar.
  $effect(() => {
    void query;
    void options;
    void open;
    void editorState.editor;
    pending = true;
    const timer = setTimeout(runSearch, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  });

  /** Opens the bar, starting from the selected text when there is some. */
  function show() {
    // Highlighting is a Milkdown plugin, so there is nothing to search with
    // when the Rust surface is the one mounted. A bar that silently finds
    // nothing is worse than saying so; project search (⇧⌘F) still works.
    if (!editorState.editor) {
      toast.info(
        'Find in document isn’t available in the Rust editor yet',
        'Use project search, or turn the Rust editor off in Settings.'
      );
      return;
    }
    editorState.editor?.action((ctx) => {
      const { state } = ctx.get(editorViewCtx);
      const { from, to } = state.selection;
      const selected = state.doc.textBetween(from, to, '\n');
      if (selected && !selected.includes('\n')) {
        query = useRegex ? selected.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') : selected;
      }
    });
    open = true;
    queueMicrotask(() => inputEl?.select());
  }

  /** Closes the bar and puts the caret back in the text, on the last match. */
  function hide() {
    open = false;
    withView((view) => {
      setFind(view, null, false);
      view.focus();
    });
  }

  /** @param {1 | -1} dir */
  function step(dir) {
    // Enter right after typing shouldn't wait for the debounce. The fresh
    // search already lands on the next match, so don't step past it.
    if (pending) {
      runSearch();
      if (dir === 1) return;
    }
    withView((view) => findStep(view, dir));
  }

  function replaceOne() {
    withView((view) => replaceCurrent(view, replacement));
  }

  function replaceEvery() {
    let count = 0;
    withView((view) => (count = replaceAll(view, replacement)));
    if (count)
      toast.success(
        `Replaced ${count} ${count === 1 ? 'match' : 'matches'}`,
        `${MOD_KEY}Z to undo.`
      );
  }

  function searchEverywhere() {
    const prefill = {
      query,
      replacement: '',
      binder: appState.ui.activeBinder,
      options: { ...options },
    };
    hide();
    if (prefill.query) appState.openProjectSearch(prefill);
    else appState.toggleProjectSearch(true);
  }

  /** @param {KeyboardEvent} e */
  function onFindKeydown(e) {
    if (e.key === 'Enter') {
      e.preventDefault();
      step(e.shiftKey ? -1 : 1);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      hide();
    }
  }

  /** @param {KeyboardEvent} e */
  function onReplaceKeydown(e) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (isMod(e)) replaceEvery();
      else replaceOne();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      hide();
    }
  }

  /**
   * ⌘F finds, ⌘⌥F finds and replaces, ⌘G / ⌘⇧G step through matches.
   * Keys go by `code`, since ⌥ turns F into ƒ on a Mac. ⌘⇧F is project search.
   * @param {KeyboardEvent} e
   */
  function onWindowKeydown(e) {
    if (!isMod(e) || e.defaultPrevented) return;
    // Leave ⌘F alone while a dialog (project search, the palette) is up.
    if (e.target instanceof Element && e.target.closest('[role="dialog"]')) return;
    if (e.code === 'KeyF' && !e.shiftKey) {
      e.preventDefault();
      if (e.altKey) showReplace = true;
      show();
    } else if (e.code === 'KeyG' && open && !e.altKey) {
      e.preventDefault();
      step(e.shiftKey ? -1 : 1);
    }
  }

  let counter = $derived.by(() => {
    if (!query) return '';
    if (error) return 'Invalid';
    if (!total) return 'No results';
    return `${current + 1} of ${total}`;
  });
</script>

<svelte:window onkeydown={onWindowKeydown} />

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
    class="h-6 min-w-6 px-1 rounded font-mono text-[0.6875rem] transition-colors
      {pressed
      ? 'bg-primary/15 text-primary'
      : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
  >
    {label}
  </button>
{/snippet}

{#snippet iconButton(
  /** @type {string} */ title,
  /** @type {() => void} */ onclick,
  /** @type {boolean} */ disabled,
  /** @type {import('svelte').Component} */ Icon
)}
  <button
    type="button"
    {title}
    aria-label={title}
    {onclick}
    {disabled}
    class="size-6 flex items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40 disabled:pointer-events-none"
  >
    <Icon strokeWidth={1.5} class="size-3.5" />
  </button>
{/snippet}

{#if open}
  <div
    role="search"
    aria-label="Find in this writing"
    class="find-bar absolute right-4 top-2 z-30 grid gap-1.5 rounded-lg border border-border/60 bg-popover/95 p-1.5 font-sans text-popover-foreground shadow-lg backdrop-blur-md"
  >
    <div class="flex items-center gap-1">
      <button
        type="button"
        onclick={() => (showReplace = !showReplace)}
        aria-label={showReplace ? 'Hide replace' : 'Show replace'}
        aria-expanded={showReplace}
        class="size-6 flex items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground"
      >
        <ChevronRightIcon
          strokeWidth={1.5}
          class="size-3.5 transition-transform {showReplace ? 'rotate-90' : ''}"
        />
      </button>
      <input
        bind:this={inputEl}
        bind:value={query}
        onkeydown={onFindKeydown}
        placeholder="Find"
        aria-label="Find"
        aria-invalid={!!error}
        title={error || undefined}
        spellcheck="false"
        autocomplete="off"
        class="w-52 h-7 rounded-md border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50
          {error ? 'border-destructive' : 'border-border'}"
      />
      {@render toggle('Aa', 'Match case', caseSensitive, () => (caseSensitive = !caseSensitive))}
      {@render toggle('ab', 'Whole word', wholeWord, () => (wholeWord = !wholeWord))}
      {@render toggle('.*', 'Regular expression', useRegex, () => (useRegex = !useRegex))}
      <span
        class="w-20 text-center text-[0.6875rem] tabular-nums {error
          ? 'text-destructive'
          : 'text-muted-foreground'}"
        aria-live="polite"
      >
        {counter}
      </span>
      {@render iconButton('Previous match (⇧↵)', () => step(-1), !total, ArrowUpIcon)}
      {@render iconButton('Next match (↵)', () => step(1), !total, ArrowDownIcon)}
      {@render iconButton(
        `Find in all writings (${MOD_KEY}⇧F)`,
        searchEverywhere,
        false,
        TextSearchIcon
      )}
      {@render iconButton('Close (Esc)', hide, false, XIcon)}
    </div>

    {#if showReplace}
      <div class="flex items-center gap-1 pl-7">
        <input
          bind:value={replacement}
          onkeydown={onReplaceKeydown}
          placeholder={useRegex ? 'Replace ($1 for groups)' : 'Replace'}
          aria-label="Replace"
          spellcheck="false"
          autocomplete="off"
          class="w-52 h-7 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        />
        <button
          type="button"
          title="Replace (↵)"
          onclick={replaceOne}
          disabled={!total}
          class="h-6 px-2 rounded text-xs text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40 disabled:pointer-events-none"
        >
          Replace
        </button>
        <button
          type="button"
          title="Replace all ({MOD_KEY}↵)"
          onclick={replaceEvery}
          disabled={!total}
          class="h-6 px-2 rounded text-xs text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40 disabled:pointer-events-none"
        >
          All
        </button>
      </div>
    {/if}
  </div>
{/if}
