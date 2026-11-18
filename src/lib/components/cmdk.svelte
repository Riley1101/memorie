<script>
  import FileTextIcon from '@lucide/svelte/icons/file-text';
  import CirclePlusIcon from '@lucide/svelte/icons/circle-plus';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import TextSearchIcon from '@lucide/svelte/icons/text-search';
  import FileOutputIcon from '@lucide/svelte/icons/file-output';
  import FileInputIcon from '@lucide/svelte/icons/file-input';
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import { updaterManager } from '$lib/runes/updater.svelte.js';
  import { openCodex } from '$lib/runes/codex.svelte.js';
  import * as Command from '$lib/components/ui/command/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { fileManager, baseOf } from '$lib/runes/fs.svelte.js';
  import { formatFileName, formatDate } from '@/utils';
  import { shortcut } from '$lib/keyboard.svelte.js';
  import { startNewWriting, sanitizeTitle, writingUrl } from '$lib/new-writing.js';
  import { toast } from '$lib/toast.js';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { resolve } from '$app/paths';
  import { dirOf } from '$lib/runes/fs.svelte.js';
  import {
    editorSession,
    editorCommands,
    commandLabel,
    commandSearchValue,
    runEditorCommand,
  } from '$lib/editor-commands.svelte.js';

  /**
   * The binder's colour as the home list shows it, so a writing reads the same
   * here as it does there. Class names are written out for Tailwind to find.
   * @type {Record<string, string>}
   */
  const BINDER_DOTS = {
    teal: 'bg-binder-teal',
    primary: 'bg-binder-clay',
    info: 'bg-binder-slate',
    success: 'bg-binder-sage',
    warning: 'bg-binder-ochre',
    accent: 'bg-binder-plum',
    muted: 'bg-muted-foreground/50',
  };

  let search = $state('');

  /**
   * Writings listed with an empty box. The palette used to render every
   * writing in the project, which is a long scroll and a lot of DOM once a
   * novel is underway; what a writer wants on open is what they touched last.
   */
  const RECENT_LIMIT = 8;
  /** Title matches shown at once, so a common word can't flood the list. */
  const TITLE_LIMIT = 40;
  /** Writings shown under "In your writing". */
  const CONTENT_LIMIT = 8;
  /** Shorter than this and a content search is all noise. */
  const MIN_CONTENT_QUERY = 3;
  /** Typing pause before the content search runs. */
  const SEARCH_DEBOUNCE_MS = 180;

  /** The writing open in the editor, or null on the home screen. */
  let openWriting = $derived(editorSession.fileName);

  // Where a writing created from the palette goes: next to the open writing
  // (as ⌘N does), else the binder open on the home screen, else the top level.
  let targetDir = $derived(openWriting !== null ? dirOf(openWriting) : (appState.ui.activeBinder ?? ''));
  let targetLabel = $derived(targetDir || 'top level');

  const COMMAND_GROUPS = /** @type {const} */ ([
    ['writing', 'This writing'],
    ['assistant', 'Assistant'],
    ['typing', 'Typing'],
  ]);

  /** The open writing's commands, by group. Empty on the home screen. */
  let commandGroups = $derived.by(() => {
    if (openWriting === null) return [];
    const commands = editorCommands();
    return COMMAND_GROUPS.map(([id, heading]) => ({
      heading,
      commands: commands.filter((c) => c.group === id),
    })).filter((g) => g.commands.length > 0);
  });

  /** @param {import('$lib/editor-commands.svelte.js').EditorCommand} command */
  function handleRunCommand(command) {
    close();
    runEditorCommand(command);
  }

  let query = $derived(search.trim());

  /** `fileManager.files` comes back newest first, so this is just the head. */
  let recentFiles = $derived(fileManager.files.slice(0, RECENT_LIMIT));

  /**
   * Writings whose title or folder matches what's typed. cmdk does its own
   * filtering on `value`, so this only has to keep the list to a sane length.
   */
  let titleMatches = $derived.by(() => {
    if (!query) return [];
    const term = query.toLowerCase();
    return fileManager.files
      .filter((f) => `${formatFileName(baseOf(f.name))} ${f.folder ?? ''}`.toLowerCase().includes(term))
      .slice(0, TITLE_LIMIT);
  });

  /**
   * Content hits from the backend. Writers look for the scene where something
   * happened, not for a filename, and this used to mean leaving the palette
   * for a separate find-in-all-writings dialog.
   * @type {{ name: string, line: number, preview: string }[]}
   */
  let contentMatches = $state([]);
  let isSearchingContent = $state(false);

  $effect(() => {
    const term = query;
    if (term.length < MIN_CONTENT_QUERY || !appState.ui.isCommandMenuOpen) {
      contentMatches = [];
      isSearchingContent = false;
      return;
    }

    let cancelled = false;
    isSearchingContent = true;
    const timer = setTimeout(async () => {
      try {
        /** @type {{ files: { name: string, matches: { line: number, preview: string, meta: boolean }[] }[] }} */
        const results = await invoke('search_project', {
          query: term,
          options: { caseSensitive: false, wholeWord: false, regex: false },
        });
        if (cancelled) return;
        // One line per writing: the palette is for getting somewhere, and
        // every match in context is what ⌘⇧F is for.
        contentMatches = results.files
          .map((file) => {
            const hit = file.matches.find((m) => !m.meta) ?? file.matches[0];
            return hit ? { name: file.name, line: hit.line, preview: hit.preview } : null;
          })
          .filter((hit) => hit !== null)
          .slice(0, CONTENT_LIMIT);
      } catch (e) {
        if (!cancelled) {
          console.error('Palette content search failed:', e);
          contentMatches = [];
        }
      } finally {
        if (!cancelled) isSearchingContent = false;
      }
    }, SEARCH_DEBOUNCE_MS);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });

  /** Content hits that a title match already covers. */
  let contentOnly = $derived.by(() => {
    const titled = new Set(titleMatches.map((f) => f.name));
    return contentMatches.filter((hit) => !titled.has(hit.name));
  });

  /**
   * Opens a writing and asks the editor to select the matched text once it
   * mounts, the same way find-in-all-writings does.
   * @param {{ name: string }} hit
   */
  function handleSelectContent(hit) {
    editorState.pendingReveal = {
      query,
      options: { caseSensitive: false, wholeWord: false, regex: false },
      occurrence: 0,
    };
    handleSelect(hit.name);
  }

  /** Typed text as a usable title, if it doesn't already name a writing. */
  let creatableTitle = $derived.by(() => {
    // ":toc" and friends are commands, not titles.
    if (query.startsWith(':')) return '';
    const title = sanitizeTitle(search);
    if (!title) return '';
    const exists = fileManager.files.some(
      (f) => formatFileName(baseOf(f.name)).toLowerCase() === title.toLowerCase()
    );
    return exists ? '' : title;
  });

  function close() {
    appState.toggleCommandMenu(false);
    search = '';
  }

  function handleSelect(fileName) {
    close();
    goto(writingUrl(fileName));
  }

  function handleCreateBlank() {
    close();
    startNewWriting(targetDir);
  }

  async function handleCreateNamed() {
    const title = creatableTitle;
    if (!title) return;
    close();
    const base = fileManager.uniqueName(targetDir, title);
    const full = targetDir ? `${targetDir}/${base}.md` : `${base}.md`;
    try {
      await fileManager.createFileAt(full, '');
      goto(writingUrl(full));
    } catch (e) {
      toast.error(`Could not create "${title}"`, e);
    }
  }

  function handleOpenCodex() {
    const binder = appState.ui.activeBinder;
    close();
    openCodex({ binder });
  }

  function handleOpenSettings() {
    close();
    goto(resolve('/settings'));
  }

  function handleCheckUpdates() {
    close();
    if (updaterManager.status === 'ready') {
      updaterManager.installAndRestart();
    } else {
      updaterManager.check();
    }
  }
</script>

<Command.Dialog
  open={appState.ui.isCommandMenuOpen}
  onOpenChange={(v) => {
    appState.toggleCommandMenu(v);
    if (!v) search = '';
  }}
  title={openWriting !== null ? 'Commands' : 'Search Writings'}
  description="Run a command, go to a writing, or type a new title to create one"
  portalProps={{}}
>
  <Command.Input
    bind:value={search}
    placeholder={openWriting !== null ? 'What would you like to do?' : 'Go to a writing, or type a new title…'}
    class=""
 />
  <Command.List class="p-1.5 max-h-105">
    <Command.Empty class="py-10 text-center text-sm text-muted-foreground">
      {#if creatableTitle}
        Press Enter to create “{creatableTitle}”.
      {:else}
        Nothing matches “{search}”.
      {/if}
    </Command.Empty>

    {#if creatableTitle}
      <Command.Group heading="Create" class="" value="">
        <Command.Item
          forceMount
          value={`create-named ${search}`}
          onSelect={handleCreateNamed}
          class=""
        >
          <CirclePlusIcon strokeWidth={1.7} class="" />
          <span class="truncate">Create “{creatableTitle}”</span>
          <span class="text-muted-foreground text-xs truncate">in {targetLabel}</span>
          <Command.Shortcut class="">↵</Command.Shortcut>
        </Command.Item>
      </Command.Group>
    {/if}

{#snippet writingRow(file)}
      <Command.Item
        value={`${formatFileName(baseOf(file.name))} ${file.folder ?? ''}`}
        onSelect={() => handleSelect(file.name)}
        class=""
      >
        <FileTextIcon strokeWidth={1.7} class="" />
        <span class="truncate">{formatFileName(baseOf(file.name))}</span>
        <span class="ml-auto flex items-center gap-1.5 shrink-0 text-xs text-muted-foreground max-w-[45%]">
          {#if file.folder}
            <span class="binder-dot size-2 rounded-[2px] {BINDER_DOTS[appState.binderColour(file.folder)] ?? 'bg-primary'}"></span>
            <span class="truncate">{file.folder}</span>
          {:else}
            <span class="tabular-nums">{formatDate(file.last_modified)}</span>
          {/if}
        </span>
      </Command.Item>
    {/snippet}

    {#each commandGroups as group (group.heading)}
      <Command.Group heading={group.heading} class="" value="">
        {#each group.commands as command (command.name)}
          <Command.Item
            value={commandSearchValue(command)}
            onSelect={() => handleRunCommand(command)}
            class=""
          >
            <command.icon strokeWidth={1.7} class="" />
            <span>{commandLabel(command)}</span>
            {#if command.shortcutLabel}
              <Command.Shortcut class="">{command.shortcutLabel}</Command.Shortcut>
            {/if}
          </Command.Item>
        {/each}
      </Command.Group>
    {/each}

    <Command.Group heading={query ? 'Writings' : 'Recent'} class="" value="">
      {#each query ? titleMatches : recentFiles as file (file.name)}
        {@render writingRow(file)}
      {/each}
    </Command.Group>

    {#if query.length >= MIN_CONTENT_QUERY && (contentOnly.length > 0 || isSearchingContent)}
      <Command.Group heading="In your writing" class="" value="">
        {#if contentOnly.length === 0}
          <p class="px-3 py-2 text-sm text-muted-foreground">Searching…</p>
        {/if}
        <!-- cmdk filters on `value`; a content hit's title usually does not
             contain the query, so the query goes in the value to keep it. -->
        {#each contentOnly as hit (hit.name)}
          <Command.Item
            value={`${search} ${hit.name}`}
            onSelect={() => handleSelectContent(hit)}
            class=""
          >
            <FileTextIcon strokeWidth={1.7} class="" />
            <span class="truncate shrink-0 max-w-[45%]">{formatFileName(baseOf(hit.name))}</span>
            <span class="ml-auto truncate text-xs text-muted-foreground">“{hit.preview}”</span>
          </Command.Item>
        {/each}
      </Command.Group>
    {/if}

    <Command.Group heading="Actions" class="" value="">
      <Command.Item value="new writing blank create :n" onSelect={handleCreateBlank} class="">
        <CirclePlusIcon strokeWidth={1.7} class="" />
        <span>New writing in {targetLabel}</span>
        <Command.Shortcut class="">{shortcut('N')}</Command.Shortcut>
      </Command.Item>
      <Command.Item value="new writing in choose location" onSelect={() => { close(); appState.toggleNewPicker(true); }} class="">
        <FolderIcon strokeWidth={1.7} class="" />
        <span>New writing in…</span>
        <Command.Shortcut class="">{shortcut('shift', 'N')}</Command.Shortcut>
      </Command.Item>
      <Command.Item value="find replace text in all writings :find" onSelect={() => { close(); appState.toggleProjectSearch(true); }} class="">
        <TextSearchIcon strokeWidth={1.7} class="" />
        <span>Find and replace in all writings</span>
        <Command.Shortcut class="">{shortcut('shift', 'F')}</Command.Shortcut>
      </Command.Item>
      {#if appState.ui.activeBinder}
        <Command.Item value="export compile binder word pdf epub" onSelect={() => { const dir = appState.ui.activeBinder ?? ''; close(); appState.openExport({ fileName: null, dir }); }} class="">
          <FileOutputIcon strokeWidth={1.7} class="" />
          <span>Export {appState.ui.activeBinder}…</span>
        </Command.Item>
      {/if}
      <Command.Item value="import word docx scrivener markdown obsidian rtf :import" onSelect={() => { const dir = appState.ui.activeBinder ?? ''; close(); appState.openImport({ dir }); }} class="">
        <FileInputIcon strokeWidth={1.7} class="" />
        <span>Import from Word, Scrivener, Markdown…</span>
      </Command.Item>
      <Command.Item value="codex characters locations factions items lore" onSelect={handleOpenCodex} class="">
        <BookUserIcon strokeWidth={1.7} class="" />
        <span>Open codex{appState.ui.activeBinder ? ` for ${appState.ui.activeBinder}` : ''}</span>
      </Command.Item>
      <Command.Item value="trash deleted restore recover undo bin :trash :deleted :restore" onSelect={() => { close(); appState.toggleTrash(true); }} class="">
        <Trash2Icon strokeWidth={1.7} class="" />
        <span>Recently deleted</span>
      </Command.Item>
      <Command.Item value="settings preferences" onSelect={handleOpenSettings} class="">
        <SettingsIcon strokeWidth={1.7} class="" />
        <span>Open settings</span>
      </Command.Item>
      <Command.Item value="check for updates upgrade version" onSelect={handleCheckUpdates} class="">
        <RefreshCwIcon strokeWidth={1.7} class="" />
        <span>{updaterManager.status === 'ready' ? 'Restart to update' : 'Check for updates'}</span>
      </Command.Item>
    </Command.Group>
  </Command.List>
  <div class="flex items-center gap-4 h-9 px-4.5 text-xs text-muted-foreground shadow-[0_-1px_0_var(--hair)]">
    <span>↩ Open</span><span>↑↓ Move</span>
    <span class="ml-auto truncate">Type to search writings, the codex and commands</span>
  </div>
</Command.Dialog>
