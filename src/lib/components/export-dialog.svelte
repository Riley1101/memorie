<script>
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager, baseOf, dirOf } from '$lib/runes/fs.svelte.js';
  import { manuscriptItems } from '$lib/export.js';
  import { formatFileName } from '$lib/utils.js';
  import { toast } from '$lib/toast.js';

  const SETTINGS_KEY = 'export';

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
  let pageSize = $state(/** @type {'a4' | 'letter'} */ (defaultPageSize()));

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
      if (saved.pageSize === 'a4' || saved.pageSize === 'letter') pageSize = saved.pageSize;
    } catch {
      // Defaults are fine.
    }
  }

  function saveSettings() {
    try {
      localStorage.setItem(
        SETTINGS_KEY,
        JSON.stringify({ format, author, writingTitles, sceneSeparator, manuscriptFormat, chapterPageBreaks, pageSize })
      );
    } catch {
      // Settings just won't be remembered.
    }
  }

  /** Default scope and title each time the dialog opens. */
  function onOpen() {
    loadSettings();
    // Offer the whole binder by default; a single writing is one click away.
    const binderScope = scopes.findLast((s) => s.kind === 'dir');
    const initial = binderScope ?? scopes[0];
    scopeId = initial?.id ?? '';
    title = initial ? formatFileName(baseOf(initial.path)) : '';
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

  function close() {
    appState.openExport(null);
  }

  function buildRequest() {
    if (!scope) return null;
    const items =
      scope.kind === 'file'
        ? [{ kind: 'document', name: scope.path, title: null }]
        : manuscriptItems(fileManager.files, fileManager.folders, scope.path, { writingTitles });
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
    };
  }

  async function runExport() {
    const request = buildRequest();
    if (!request) return;
    if (!request.items.some((i) => i.kind === 'document')) {
      toast.info('Nothing to export', 'This folder has no writings in it yet.');
      return;
    }
    busy = true;
    saveSettings();
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
</script>

<Dialog.Root
  {open}
  onOpenChange={(v) => {
    if (!v) close();
  }}
>
  <Dialog.Content class="sm:max-w-[480px] font-sans {appState.ui.theme}" portalProps={{}}>
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
      class="grid gap-4"
      onsubmit={(e) => {
        e.preventDefault();
        runExport();
      }}
    >
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
          <input
            bind:value={title}
            placeholder="Untitled"
            class="h-9 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          />
        </label>
        <label class="grid gap-1.5">
          <span class="text-xs font-medium text-muted-foreground">Author</span>
          <input
            bind:value={author}
            placeholder="Optional"
            class="h-9 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          />
        </label>
      </div>

      <details class="group rounded-md border border-border/50 px-3 py-2">
        <summary class="cursor-pointer text-xs font-medium text-muted-foreground select-none">
          Layout options
        </summary>
        <div class="grid gap-3 pt-3">
          {#if !isSingle}
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" bind:checked={writingTitles} />
              Use writing titles as headings
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" bind:checked={chapterPageBreaks} />
              Start each top-level folder on a new page
            </label>
            <label class="grid gap-1.5">
              <span class="text-sm">Between writings</span>
              <input
                bind:value={sceneSeparator}
                placeholder="Nothing"
                class="h-8 w-40 rounded-md border border-border bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              />
            </label>
          {/if}
          {#if format === 'pdf'}
            <label class="flex items-center justify-between gap-2 text-sm">
              Page size
              <select
                bind:value={pageSize}
                class="h-8 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              >
                <option value="a4">A4</option>
                <option value="letter">US Letter</option>
              </select>
            </label>
          {/if}
          {#if format === 'docx' || format === 'pdf'}
            <label class="flex items-start gap-2 text-sm">
              <input type="checkbox" class="mt-1" bind:checked={manuscriptFormat} />
              <span>
                Standard manuscript format
                <span class="block text-xs text-muted-foreground">
                  Times New Roman 12pt, double spaced, indented paragraphs.
                </span>
              </span>
            </label>
          {/if}
        </div>
      </details>

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
