<script>
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager, baseOf, dirOf } from '$lib/runes/fs.svelte.js';
  import { manuscriptItems, manuscriptOutline, isExcluded } from '$lib/export.js';
  import { parseWriting } from '$lib/front-matter.js';
  import { formatFileName } from '$lib/utils.js';
  import { toast } from '$lib/toast.js';

  /**
   * Settings for the writer (format, author, page setup) are kept in this
   * browser; settings for the book are kept in the binder's .compile.json so
   * they travel with it. The last-used book settings seed binders that have none.
   */
  const SETTINGS_KEY = 'export';

  /** @typedef {'copyright' | 'dedication' | 'epigraph' | 'foreword' | 'afterword' | 'acknowledgments' | 'about_the_author'} MatterKind */
  /** @typedef {{ enabled: boolean, text: string, name: string }} MatterSlot */

  /** @type {{ kind: MatterKind, place: 'front' | 'back', label: string, placeholder: string }[]} */
  const MATTER = [
    { kind: 'copyright', place: 'front', label: 'Copyright', placeholder: 'Copyright © {year} {author}\n\nAll rights reserved.' },
    { kind: 'dedication', place: 'front', label: 'Dedication', placeholder: 'For …' },
    { kind: 'epigraph', place: 'front', label: 'Epigraph', placeholder: '“A quotation.”\n\n— Its author' },
    { kind: 'foreword', place: 'front', label: 'Foreword', placeholder: '' },
    { kind: 'afterword', place: 'back', label: 'Afterword', placeholder: '' },
    { kind: 'acknowledgments', place: 'back', label: 'Acknowledgments', placeholder: 'Thank you to …' },
    { kind: 'about_the_author', place: 'back', label: 'About the Author', placeholder: '{author} lives in …' },
  ];

  /** @returns {Record<MatterKind, MatterSlot>} */
  function emptyMatter() {
    return /** @type {Record<MatterKind, MatterSlot>} */ (
      Object.fromEntries(MATTER.map((m) => [m.kind, { enabled: false, text: '', name: '' }]))
    );
  }

  /** @type {{ template: string, label: string }[]} */
  const HEADING_PRESETS = [
    { template: '', label: 'Folder names as they are' },
    { template: 'Chapter {n}', label: 'Chapter 1' },
    { template: 'Chapter {n}: {title}', label: 'Chapter 1: Title' },
    { template: 'Chapter {word}', label: 'Chapter One' },
    { template: '{roman}. {title}', label: 'I. Title' },
    { template: '{n}', label: '1' },
  ];

  /** @typedef {'docx' | 'pdf' | 'epub' | 'html' | 'markdown'} Format */

  /** @type {{ id: Format, label: string, hint: string }[]} */
  const FORMATS = [
    { id: 'docx', label: 'Word', hint: 'For agents, editors and Word or Pages.' },
    { id: 'pdf', label: 'PDF', hint: 'Typeset and ready to read, print, or send.' },
    { id: 'epub', label: 'EPUB', hint: 'E-book for Apple Books, Kobo, or uploading to KDP.' },
    { id: 'html', label: 'Web page', hint: 'A single HTML file.' },
    { id: 'markdown', label: 'Markdown', hint: 'Everything combined into one .md file.' },
  ];

  let target = $derived(appState.ui.exportTarget);
  let open = $derived(target !== null);

  /**
   * What can be exported from here: the open writing, then every folder above
   * it up to its binder. From the home screen, just the chosen binder.
   */
  let scopes = $derived.by(() => {
    if (!target) return [];
    /** @type {{ id: string, label: string, kind: 'file' | 'dir', path: string }[]} */
    const list = [];
    if (target.fileName) {
      list.push({
        id: `file:${target.fileName}`,
        label: `This writing — ${formatFileName(baseOf(target.fileName))}`,
        kind: 'file',
        path: target.fileName,
      });
    }
    let dir = target.dir;
    while (dir) {
      const count = fileManager.filesUnder(dir).length;
      const isBinder = !dir.includes('/');
      list.push({
        id: `dir:${dir}`,
        label: `${isBinder ? 'Whole binder' : 'Folder'} — ${baseOf(dir)} (${count} ${count === 1 ? 'writing' : 'writings'})`,
        kind: 'dir',
        path: dir,
      });
      dir = dirOf(dir);
    }
    return list;
  });

  let scopeId = $state('');
  let format = $state(/** @type {Format} */ ('docx'));
  let title = $state('');
  let author = $state('');
  let writingTitles = $state(true);
  let sceneSeparator = $state('* * *');
  let manuscriptFormat = $state(false);
  let chapterPageBreaks = $state(true);
  let tableOfContents = $state(false);
  let runningHead = $state(false);
  let pageSize = $state(/** @type {'a4' | 'letter'} */ (defaultPageSize()));
  let chapterHeading = $state('');
  let customHeading = $state(false);
  /** Statuses whose writings are left out, lower-cased. */
  let excludeStatuses = $state(/** @type {string[]} */ ([]));
  /** Content-relative paths of left-out folders and writings. */
  let excluded = $state(/** @type {Set<string>} */ (new Set()));
  /** Status of each writing in scope, read when a folder is chosen. */
  let statusOf = $state(/** @type {Map<string, string>} */ (new Map()));
  let matter = $state(emptyMatter());
  /** The binder whose .compile.json has been read; nothing is saved before that. */
  let loadedBinder = $state('');
  let panel = $state(/** @type {'contents' | 'matter'} */ ('contents'));
  /** Book settings as last read or written, to skip saving when nothing changed. */
  let bookSnapshot = '';

  /** US and Canadian writers expect Letter; everyone else A4. */
  function defaultPageSize() {
    try {
      const region = new Intl.Locale(navigator.language).maximize().region;
      return region === 'US' || region === 'CA' ? 'letter' : 'a4';
    } catch {
      return 'a4';
    }
  }
  let busy = $state(false);

  let scope = $derived(scopes.find((s) => s.id === scopeId) ?? scopes[0]);

  function loadSettings() {
    try {
      const saved = JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? '{}');
      if (FORMATS.some((f) => f.id === saved.format)) format = saved.format;
      if (typeof saved.author === 'string') author = saved.author;
      if (typeof saved.writingTitles === 'boolean') writingTitles = saved.writingTitles;
      if (typeof saved.sceneSeparator === 'string') sceneSeparator = saved.sceneSeparator;
      if (typeof saved.manuscriptFormat === 'boolean') manuscriptFormat = saved.manuscriptFormat;
      if (typeof saved.chapterPageBreaks === 'boolean') chapterPageBreaks = saved.chapterPageBreaks;
      if (typeof saved.tableOfContents === 'boolean') tableOfContents = saved.tableOfContents;
      if (typeof saved.runningHead === 'boolean') runningHead = saved.runningHead;
      if (saved.pageSize === 'a4' || saved.pageSize === 'letter') pageSize = saved.pageSize;
      if (typeof saved.chapterHeading === 'string') chapterHeading = saved.chapterHeading;
      if (Array.isArray(saved.excludeStatuses)) {
        excludeStatuses = saved.excludeStatuses.filter((/** @type {unknown} */ s) => typeof s === 'string');
      }
    } catch {
      // Defaults are fine.
    }
  }

  function saveSettings() {
    try {
      localStorage.setItem(
        SETTINGS_KEY,
        JSON.stringify({
          format,
          author,
          writingTitles,
          sceneSeparator,
          manuscriptFormat,
          chapterPageBreaks,
          tableOfContents,
          runningHead,
          pageSize,
          chapterHeading,
          excludeStatuses,
        })
      );
    } catch {
      // Settings just won't be remembered.
    }
    saveBook();
  }

  /** Book settings as saved in .compile.json. Paths are relative to the binder. */
  function bookSettings(/** @type {string} */ binder) {
    const relative = (/** @type {string} */ p) => p.slice(binder.length + 1);
    return {
      version: 1,
      ...(scope?.path === binder ? { title } : {}),
      writingTitles,
      sceneSeparator,
      chapterPageBreaks,
      chapterHeading,
      tableOfContents,
      excludeStatuses,
      excluded: [...excluded].filter((p) => p.startsWith(`${binder}/`)).map(relative),
      matter: Object.fromEntries(
        MATTER.map(({ kind }) => [
          kind,
          { ...matter[kind], name: matter[kind].name.startsWith(`${binder}/`) ? relative(matter[kind].name) : '' },
        ])
      ),
    };
  }

  function saveBook() {
    if (scope?.kind !== 'dir') return;
    const binder = binderOf(scope.path);
    if (loadedBinder !== binder) return;
    const settings = bookSettings(binder);
    const snapshot = JSON.stringify(settings);
    if (snapshot === bookSnapshot) return;
    bookSnapshot = snapshot;
    invoke('write_compile_settings', { binder, settings }).catch((e) =>
      console.warn('Could not save export settings', e)
    );
  }

  /** @param {string} binder */
  async function loadBook(binder) {
    matter = emptyMatter();
    excluded = new Set();
    loadedBinder = '';
    /** @type {any} */
    let saved = {};
    try {
      const json = await invoke('read_compile_settings', { binder });
      if (typeof json === 'string') saved = JSON.parse(json);
    } catch (e) {
      console.warn('Could not read export settings', e);
    }
    if (!target || binderOf(scope?.path ?? '') !== binder) return;
    const absolute = (/** @type {string} */ p) => `${binder}/${p}`;
    if (typeof saved.title === 'string' && saved.title.trim() && scope?.path === binder) title = saved.title;
    if (typeof saved.writingTitles === 'boolean') writingTitles = saved.writingTitles;
    if (typeof saved.sceneSeparator === 'string') sceneSeparator = saved.sceneSeparator;
    if (typeof saved.chapterPageBreaks === 'boolean') chapterPageBreaks = saved.chapterPageBreaks;
    if (typeof saved.tableOfContents === 'boolean') tableOfContents = saved.tableOfContents;
    if (typeof saved.chapterHeading === 'string') {
      chapterHeading = saved.chapterHeading;
      customHeading = !HEADING_PRESETS.some((p) => p.template === chapterHeading);
    }
    if (Array.isArray(saved.excludeStatuses)) {
      excludeStatuses = saved.excludeStatuses.filter((/** @type {unknown} */ s) => typeof s === 'string');
    }
    if (Array.isArray(saved.excluded)) {
      excluded = new Set(saved.excluded.filter((/** @type {unknown} */ p) => typeof p === 'string').map(absolute));
    }
    if (saved.matter && typeof saved.matter === 'object') {
      const next = emptyMatter();
      for (const { kind } of MATTER) {
        const slot = saved.matter[kind];
        if (!slot || typeof slot !== 'object') continue;
        next[kind] = {
          enabled: slot.enabled === true,
          text: typeof slot.text === 'string' ? slot.text : '',
          name: typeof slot.name === 'string' && slot.name ? absolute(slot.name) : '',
        };
      }
      matter = next;
    }
    loadedBinder = binder;
    bookSnapshot = JSON.stringify(bookSettings(binder));
  }

  /** @param {string} path */
  function binderOf(path) {
    return path.split('/')[0];
  }

  /** Default scope and title each time the dialog opens. */
  function onOpen() {
    loadSettings();
    customHeading = !HEADING_PRESETS.some((p) => p.template === chapterHeading);
    // Offer the whole binder by default; a single writing is one click away.
    const binderScope = scopes.findLast((s) => s.kind === 'dir');
    const initial = binderScope ?? scopes[0];
    scopeId = initial?.id ?? '';
    title = initial ? formatFileName(baseOf(initial.path)) : '';
    panel = 'contents';
    if (initial?.kind === 'dir') loadBook(binderOf(initial.path));
  }

  /** @param {string} id */
  function chooseScope(id) {
    const previous = scope;
    scopeId = id;
    // Follow the scope with the title unless the writer typed their own.
    if (previous && title === formatFileName(baseOf(previous.path))) {
      const next = scopes.find((s) => s.id === id);
      if (next) title = formatFileName(baseOf(next.path));
    }
  }

  // Opening happens from outside (commands, menus), so reset on the target.
  $effect(() => {
    if (target) untrack(onOpen);
  });

  let outline = $derived(
    open && scope?.kind === 'dir' ? manuscriptOutline(fileManager.files, fileManager.folders, scope.path) : []
  );

  // Read each writing's status so the filter can offer the ones in use.
  $effect(() => {
    const names = outline.filter((e) => e.kind === 'file').map((e) => e.path);
    if (names.length === 0) {
      statusOf = new Map();
      return;
    }
    let cancelled = false;
    invoke('read_files', { names })
      .then((/** @type {[string, { Ok?: string } | string][]} */ results) => {
        if (cancelled) return;
        /** @type {Map<string, string>} */
        const next = new Map();
        for (const [name, result] of results) {
          const content = typeof result === 'string' ? result : result?.Ok;
          const status = typeof content === 'string' ? parseWriting(content).meta.status?.trim() : '';
          if (status) next.set(name, status);
        }
        statusOf = next;
      })
      .catch(() => {
        // The status filter just won't show; exporting still works.
      });
    return () => {
      cancelled = true;
    };
  });

  /** Statuses in use in this scope, with how many writings have each. */
  let statusCounts = $derived.by(() => {
    /** @type {Map<string, { label: string, count: number }>} */
    const counts = new Map();
    for (const status of statusOf.values()) {
      const key = status.toLowerCase();
      const entry = counts.get(key) ?? { label: status, count: 0 };
      entry.count += 1;
      counts.set(key, entry);
    }
    return [...counts].sort(([a], [b]) => a.localeCompare(b));
  });

  /** Writings used as front or back matter, by name, with the page they fill. */
  let matterUse = $derived(
    new Map(
      MATTER.filter(({ kind }) => matter[kind].enabled && matter[kind].name).map(({ kind, label }) => [
        matter[kind].name,
        label,
      ])
    )
  );

  /** @param {string} path */
  function isLeftOut(path) {
    if (isExcluded(path, excluded) || matterUse.has(path)) return true;
    const status = statusOf.get(path);
    return !!status && excludeStatuses.includes(status.toLowerCase());
  }

  let fileEntries = $derived(outline.filter((e) => e.kind === 'file'));
  let includedCount = $derived(fileEntries.filter((e) => !isLeftOut(e.path)).length);

  /** @param {string} path */
  function toggleExcluded(path) {
    const next = new Set(excluded);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    excluded = next;
  }

  /** @param {string} status - lower-cased */
  function toggleStatus(status) {
    excludeStatuses = excludeStatuses.includes(status)
      ? excludeStatuses.filter((s) => s !== status)
      : [...excludeStatuses, status];
  }

  function close() {
    if (scope) saveSettings();
    appState.openExport(null);
  }

  /** @param {'front' | 'back'} place */
  function matterItems(place) {
    return MATTER.filter((m) => m.place === place && matter[m.kind].enabled).map(({ kind }) => ({
      kind,
      text: matter[kind].text,
      name: matter[kind].name || null,
    }));
  }

  function buildRequest() {
    if (!scope) return null;
    const items =
      scope.kind === 'file'
        ? [{ kind: 'document', name: scope.path, title: null }]
        : manuscriptItems(fileManager.files, fileManager.folders, scope.path, {
            writingTitles,
            // Writings used as matter aren't repeated in the body.
            excluded: new Set([...excluded, ...matterUse.keys()]),
          });
    return {
      format,
      title: title.trim(),
      author: author.trim(),
      items,
      scene_separator: sceneSeparator,
      manuscript_format: manuscriptFormat,
      chapter_page_breaks: chapterPageBreaks,
      page_size: pageSize,
      lang: (navigator.language || 'en').split('-')[0].toLowerCase(),
      chapter_heading: scope.kind === 'dir' ? chapterHeading.trim() : '',
      exclude_statuses: scope.kind === 'dir' ? excludeStatuses : [],
      table_of_contents: scope.kind === 'dir' && hasContents && tableOfContents,
      running_head: hasRunningHead && runningHead,
      front_matter: scope.kind === 'dir' ? matterItems('front') : [],
      back_matter: scope.kind === 'dir' ? matterItems('back') : [],
    };
  }

  async function runExport() {
    const request = buildRequest();
    if (!request) return;
    if (!request.items.some((i) => i.kind === 'document')) {
      toast.info(
        'Nothing to export',
        fileEntries.length > 0 ? 'Everything in this folder is left out.' : 'This folder has no writings in it yet.'
      );
      return;
    }
    busy = true;
    try {
      // Export what's on screen, not what was last autosaved.
      if (editorState.flushSave) await editorState.flushSave();

      /** @type {string} */
      const path = await invoke('export_manuscript', { request });
      close();
      toast.successWithAction(
        `Exported “${baseOf(path)}”`,
        'Show',
        () => revealItemInDir(path).catch((e) => toast.error('Could not show the file', e)),
        'Saved to your Downloads folder.'
      );
    } catch (e) {
      toast.error('Export failed', e);
    } finally {
      busy = false;
    }
  }

  let formatHint = $derived(FORMATS.find((f) => f.id === format)?.hint ?? '');
  let isSingle = $derived(scope?.kind === 'file');
  let showContents = $derived(!isSingle && outline.length > 0);
  let hasContents = $derived(format === 'docx' || format === 'pdf' || format === 'epub');
  let hasRunningHead = $derived(format === 'docx' || format === 'pdf');
  let matterCount = $derived(MATTER.filter(({ kind }) => matter[kind].enabled).length);
</script>

<Dialog.Root
  {open}
  onOpenChange={(v) => {
    if (!v) close();
  }}
>
  <Dialog.Content
    class="{showContents ? 'sm:max-w-[880px]' : 'sm:max-w-[520px]'} max-h-[90vh] flex flex-col font-sans {appState.ui.theme}"
    portalProps={{}}
  >
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal font-writer">Export</Dialog.Title>
      <Dialog.Description class="text-muted-foreground">
        Combine writings into one file for sharing, submitting, or publishing.
      </Dialog.Description>
    </Dialog.Header>

    {#if scopes.length === 0}
      <p class="text-sm text-muted-foreground">
        Nothing to export yet. Write something first, or put this writing in a binder to export several together.
      </p>
      <Dialog.Footer class="">
        <Button onclick={close} disabled={false}>OK</Button>
      </Dialog.Footer>
    {:else}
    <form
      class="flex min-h-0 flex-col gap-4"
      onsubmit={(e) => {
        e.preventDefault();
        runExport();
      }}
    >
      <div class="grid min-h-0 gap-6 {showContents ? 'md:grid-cols-2' : ''}">
        <div class="grid content-start gap-4 min-h-0 overflow-y-auto pr-1 -mr-1">
          <label class="grid gap-1.5">
            <span class="text-xs font-medium text-muted-foreground">What</span>
            <select
              value={scope?.id}
              onchange={(e) => chooseScope(e.currentTarget.value)}
              class="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            >
              {#each scopes as s (s.id)}
                <option value={s.id}>{s.label}</option>
              {/each}
            </select>
          </label>

          <fieldset class="grid gap-1.5">
            <legend class="text-xs font-medium text-muted-foreground mb-1.5">Format</legend>
            <div class="flex flex-wrap gap-1 bg-muted/40 p-1 rounded-md border border-border/50">
              {#each FORMATS as f (f.id)}
                <Button
                  type="button"
                  variant={format === f.id ? 'secondary' : 'ghost'}
                  size="sm"
                  class="flex-1"
                  aria-pressed={format === f.id}
                  onclick={() => (format = f.id)}
                  disabled={false}
                >
                  {f.label}
                </Button>
              {/each}
            </div>
            <p class="text-xs text-muted-foreground">{formatHint}</p>
          </fieldset>

          <div class="grid grid-cols-2 gap-3">
            <label class="grid gap-1.5">
              <span class="text-xs font-medium text-muted-foreground">Title</span>
              <input bind:value={title} placeholder="Untitled" class="h-9 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50" />
            </label>
            <label class="grid gap-1.5">
              <span class="text-xs font-medium text-muted-foreground">Author</span>
              <input bind:value={author} placeholder="Optional" class="h-9 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50" />
            </label>
          </div>

          {#if !isSingle}
            <section class="grid gap-3">
              <h3 class="text-xs font-medium text-muted-foreground">Chapters</h3>
              <label class="flex items-center justify-between gap-2 text-sm">
                Headings
                <select
                  value={customHeading ? 'custom' : chapterHeading}
                  onchange={(e) => {
                    const value = e.currentTarget.value;
                    customHeading = value === 'custom';
                    if (!customHeading) chapterHeading = value;
                  }}
                  class="h-8 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                >
                  {#each HEADING_PRESETS as p (p.template)}
                    <option value={p.template}>{p.label}</option>
                  {/each}
                  <option value="custom">Custom…</option>
                </select>
              </label>
              {#if customHeading}
                <div class="grid gap-1">
                  <input bind:value={chapterHeading} placeholder={'Chapter {n}: {title}'} class="h-8 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50" />
                  <span class="text-xs text-muted-foreground">
                    {'{n}'} 3, {'{word}'} Three, {'{roman}'} III, {'{title}'} the folder name.
                  </span>
                </div>
              {/if}
              {#if chapterHeading.trim()}
                <span class="-mt-1.5 text-xs text-muted-foreground">
                  Innermost folders are chapters, or each writing if there are no folders.
                </span>
              {/if}
              <label class="flex items-center gap-2 text-sm">
                <input type="checkbox" bind:checked={writingTitles} />
                Use writing titles as headings
              </label>
              <label class="flex items-center gap-2 text-sm">
                <input type="checkbox" bind:checked={chapterPageBreaks} />
                Start each top-level folder on a new page
              </label>
              {#if hasContents}
                <label class="flex items-center gap-2 text-sm">
                  <input type="checkbox" bind:checked={tableOfContents} />
                  Table of contents after the title page
                </label>
              {/if}
              <label class="flex items-center justify-between gap-2 text-sm">
                Between writings
                <input bind:value={sceneSeparator} placeholder="Nothing" class="h-8 w-32 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50" />
              </label>
            </section>
          {/if}

          {#if format === 'pdf' || format === 'docx'}
            <section class="grid gap-3">
              <h3 class="text-xs font-medium text-muted-foreground">Page</h3>
              {#if format === 'pdf'}
                <label class="flex items-center justify-between gap-2 text-sm">
                  Page size
                  <select bind:value={pageSize} class="h-8 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50">
                    <option value="a4">A4</option>
                    <option value="letter">US Letter</option>
                  </select>
                </label>
              {/if}
              <label class="flex items-start gap-2 text-sm">
                <input type="checkbox" class="mt-1" bind:checked={manuscriptFormat} />
                <span>
                  Standard manuscript format
                  <span class="block text-xs text-muted-foreground">
                    Times New Roman 12pt, double spaced, indented paragraphs.
                  </span>
                </span>
              </label>
              <label class="flex items-start gap-2 text-sm">
                <input type="checkbox" class="mt-1" bind:checked={runningHead} />
                <span>
                  Running head
                  <span class="block text-xs text-muted-foreground">
                    {[author.trim().split(/\s+/).at(-1), title.trim().toUpperCase()].filter(Boolean).join(' / ') ||
                      'Page number'}{author.trim() || title.trim() ? ' / page' : ''} at the top of each page.
                  </span>
                </span>
              </label>
            </section>
          {/if}
        </div>

        {#if showContents}
          <section class="flex min-h-0 flex-col gap-3 rounded-md border border-border/50 p-3 md:max-h-[60vh]">
            <div class="flex gap-1 rounded-md bg-muted/40 p-1" role="tablist">
              <Button
                type="button"
                role="tab"
                variant={panel === 'contents' ? 'secondary' : 'ghost'}
                size="sm"
                class="flex-1"
                aria-selected={panel === 'contents'}
                onclick={() => (panel = 'contents')}
                disabled={false}
              >
                Contents
                <span class="text-muted-foreground tabular-nums">{includedCount}/{fileEntries.length}</span>
              </Button>
              <Button
                type="button"
                role="tab"
                variant={panel === 'matter' ? 'secondary' : 'ghost'}
                size="sm"
                class="flex-1"
                aria-selected={panel === 'matter'}
                onclick={() => (panel = 'matter')}
                disabled={false}
              >
                Front &amp; back matter
                {#if matterCount > 0}<span class="text-muted-foreground tabular-nums">{matterCount}</span>{/if}
              </Button>
            </div>

            {#if panel === 'contents'}
              {#if statusCounts.length > 0}
                <div class="grid gap-1.5">
                  <span class="text-sm">Leave out writings marked</span>
                  <div class="flex flex-wrap gap-1">
                    {#each statusCounts as [key, { label, count }] (key)}
                      <Button
                        type="button"
                        variant={excludeStatuses.includes(key) ? 'secondary' : 'outline'}
                        size="sm"
                        class="h-7 text-xs {excludeStatuses.includes(key) ? 'line-through' : ''}"
                        aria-pressed={excludeStatuses.includes(key)}
                        onclick={() => toggleStatus(key)}
                        disabled={false}
                      >
                        {label} <span class="text-muted-foreground tabular-nums">{count}</span>
                      </Button>
                    {/each}
                  </div>
                </div>
              {/if}
              <ul class="min-h-0 flex-1 overflow-y-auto grid content-start gap-0.5 text-sm max-h-72 md:max-h-none" aria-label="Writings to include">
                {#each outline as entry (entry.path)}
                  {@const parentOut = isExcluded(entry.path.slice(0, entry.path.lastIndexOf('/')), excluded)}
                  {@const statusOut = entry.kind === 'file' && !parentOut && !excluded.has(entry.path) && isLeftOut(entry.path)}
                  {@const note = matterUse.has(entry.path) ? `Used as ${matterUse.get(entry.path)}` : statusOf.get(entry.path)}
                  <li style="padding-left: {(entry.depth - 1) * 1}rem">
                    <label class="flex items-center gap-2 py-0.5 {parentOut || statusOut ? 'opacity-50' : ''}">
                      <input
                        type="checkbox"
                        checked={!excluded.has(entry.path)}
                        disabled={parentOut}
                        onchange={() => toggleExcluded(entry.path)}
                      />
                      <span class="truncate {entry.kind === 'folder' ? 'font-medium' : ''}">{entry.label}</span>
                      {#if entry.kind === 'file' && note}
                        <span class="ml-auto shrink-0 text-xs text-muted-foreground">{note}</span>
                      {/if}
                    </label>
                  </li>
                {/each}
              </ul>
            {:else}
              <div class="min-h-0 flex-1 overflow-y-auto grid content-start gap-3 max-h-96 md:max-h-none pr-1 -mr-1">
                <p class="text-xs text-muted-foreground">
                  Pages before and after the chapters, each on its own. Write them here or use a writing from the
                  binder; {'{year}'}, {'{author}'} and {'{title}'} are filled in.
                </p>
                {#each ['front', 'back'] as place (place)}
                  <div class="grid gap-2">
                    <h4 class="text-xs font-medium text-muted-foreground">
                      {place === 'front' ? 'Before the contents' : 'After the last chapter'}
                    </h4>
                    {#each MATTER.filter((m) => m.place === place) as m (m.kind)}
                      {@const slot = matter[m.kind]}
                      <div class="grid gap-1.5">
                        <div class="flex items-center justify-between gap-2">
                          <label class="flex items-center gap-2 text-sm">
                            <input
                              type="checkbox"
                              checked={slot.enabled}
                              onchange={(e) => {
                                slot.enabled = e.currentTarget.checked;
                                if (slot.enabled && m.kind === 'copyright' && !slot.text && !slot.name) {
                                  slot.text = m.placeholder;
                                }
                              }}
                            />
                            {m.label}
                          </label>
                          {#if slot.enabled}
                            <select
                              value={slot.name}
                              onchange={(e) => (slot.name = e.currentTarget.value)}
                              aria-label="{m.label} source"
                              class="h-7 max-w-[55%] text-xs rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                            >
                              <option value="">Write here</option>
                              {#each fileEntries as f (f.path)}
                                <option value={f.path}>{f.label}</option>
                              {/each}
                            </select>
                          {/if}
                        </div>
                        {#if slot.enabled && !slot.name}
                          <textarea
                            bind:value={slot.text}
                            rows={m.kind === 'foreword' || m.kind === 'afterword' ? 5 : 3}
                            placeholder={m.placeholder}
                            aria-label={m.label}
                            class="py-1.5 resize-y rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                          ></textarea>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {/each}
              </div>
            {/if}
          </section>
        {/if}
      </div>

      <Dialog.Footer class="">
        <Button type="button" variant="ghost" onclick={close} disabled={busy}>Cancel</Button>
        <Button type="submit" disabled={busy || !scope}>
          {busy ? 'Exporting…' : 'Export'}
        </Button>
      </Dialog.Footer>
    </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
