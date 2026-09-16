<script>
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openPicker } from '@tauri-apps/plugin-dialog';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { memoryManager } from '$lib/runes/memory.svelte.js';
  import { fileManager, baseOf } from '$lib/runes/fs.svelte.js';
  import { toast } from '$lib/toast.js';
  import FileTextIcon from '@lucide/svelte/icons/file-text';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import XIcon from '@lucide/svelte/icons/x';

  /** @typedef {{ created: string[], folders: string[], warnings: string[] }} ImportReport */

  let target = $derived(appState.ui.importTarget);
  let open = $derived(target !== null);

  /** Absolute paths picked by the writer. @type {string[]} */
  let sources = $state([]);
  let destination = $state('');
  let splitDocx = $state(true);
  let busy = $state(false);
  /** Shown after an import that had something to say. @type {ImportReport | null} */
  let report = $state(null);

  let locations = $derived.by(() => {
    const dirs = [...new Set([...fileManager.binders, ...fileManager.folders])].sort((a, b) =>
      a.localeCompare(b)
    );
    return [{ dir: '', label: 'Top level' }, ...dirs.map((dir) => ({ dir, label: dir.replaceAll('/', ' / ') }))];
  });

  let hasDocx = $derived(sources.some((s) => s.toLowerCase().endsWith('.docx')));

  $effect(() => {
    if (target) {
      untrack(() => {
        sources = [];
        report = null;
        destination = target?.dir ?? '';
      });
    }
  });

  function close() {
    appState.openImport(null);
  }

  /** @param {string | string[] | null} picked */
  function addSources(picked) {
    if (!picked) return;
    const list = Array.isArray(picked) ? picked : [picked];
    sources = [...new Set([...sources, ...list])];
  }

  async function pickFiles() {
    try {
      addSources(
        await openPicker({
          multiple: true,
          title: 'Import writing',
          filters: [
            { name: 'Writing', extensions: ['docx', 'scriv', 'md', 'markdown', 'txt', 'rtf'] },
          ],
        })
      );
    } catch (e) {
      toast.error('Could not open the file picker', e);
    }
  }

  async function pickFolder() {
    try {
      addSources(await openPicker({ directory: true, multiple: true, title: 'Import a folder of notes' }));
    } catch (e) {
      toast.error('Could not open the folder picker', e);
    }
  }

  /** @param {string} path */
  function removeSource(path) {
    sources = sources.filter((s) => s !== path);
  }

  /** @param {string} path */
  function describe(path) {
    const lower = path.toLowerCase();
    if (lower.endsWith('.scriv')) return 'Scrivener project';
    if (lower.endsWith('.docx')) return 'Word document';
    if (lower.endsWith('.rtf')) return 'Rich text';
    if (/\.(md|markdown|txt)$/.test(lower)) return 'Text';
    return 'Folder';
  }

  async function runImport() {
    if (sources.length === 0) return;
    busy = true;
    try {
      /** @type {ImportReport} */
      const result = await invoke('import_sources', {
        sources: $state.snapshot(sources),
        targetDir: destination,
        splitDocx,
      });
      await Promise.all([fileManager.getRecents(), fileManager.getBinders(), fileManager.getFolders()]);
      if (configManager.config?.ai_enabled && result.created.length > 0) {
        memoryManager.reindexNotes();
      }

      const n = result.created.length;
      if (n === 0 && result.warnings.length > 0) {
        report = result;
        return;
      }
      toast.success(`Imported ${n} ${n === 1 ? 'writing' : 'writings'}`);
      if (result.warnings.length > 0) {
        report = result;
      } else {
        close();
        revealImported(result);
      }
    } catch (e) {
      toast.error('Import failed', e);
    } finally {
      busy = false;
    }
  }

  /** Takes the writer to what they just imported. @param {ImportReport} result */
  function revealImported(result) {
    if (result.created.length === 1 && result.folders.length === 0) {
      goto(resolve(`/${encodeURIComponent(result.created[0])}`));
      return;
    }
    const first = result.folders[0] ?? destination;
    const binder = first.split('/')[0];
    if (binder) {
      appState.ui.activeBinder = binder;
      goto(resolve('/'));
    }
  }
</script>

<Dialog.Root {open} onOpenChange={(v) => !v && close()}>
  <Dialog.Content class="sm:max-w-[520px] font-sans {appState.ui.theme}" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal font-writer">Import</Dialog.Title>
      <Dialog.Description class="text-muted-foreground">
        Word documents, Scrivener projects, RTF, Markdown, or a whole folder of notes (Obsidian vaults
        included). Nothing is changed in the originals.
      </Dialog.Description>
    </Dialog.Header>

    {#if report}
      <div class="grid gap-3">
        <p class="text-sm">
          Imported {report.created.length}
          {report.created.length === 1 ? 'writing' : 'writings'}.
          {#if report.warnings.length}Some things need a look:{/if}
        </p>
        <ul class="grid gap-1.5 max-h-60 overflow-y-auto text-sm text-muted-foreground list-disc pl-5">
          {#each report.warnings as warning, i (i)}
            <li>{warning}</li>
          {/each}
        </ul>
        <Dialog.Footer class="">
          {#if report.created.length > 0}
            <Button
              onclick={() => {
                const done = report;
                close();
                if (done) revealImported(done);
              }}
              disabled={false}
            >
              Show imported writing
            </Button>
          {:else}
            <Button onclick={() => (report = null)} disabled={false}>Back</Button>
          {/if}
        </Dialog.Footer>
      </div>
    {:else}
      <div class="grid gap-4">
        <div class="grid gap-2">
          <div class="flex gap-2">
            <Button variant="outline" class="flex-1" onclick={pickFiles} disabled={busy}>
              <FileTextIcon strokeWidth={1.5} />
              Choose files…
            </Button>
            <Button variant="outline" class="flex-1" onclick={pickFolder} disabled={busy}>
              <FolderIcon strokeWidth={1.5} />
              Choose folder…
            </Button>
          </div>
          {#if sources.length}
            <ul class="grid gap-1 max-h-48 overflow-y-auto rounded-md border border-border/50 p-1">
              {#each sources as source (source)}
                <li class="flex items-center gap-2 rounded px-2 py-1 hover:bg-muted/40">
                  <span class="min-w-0 flex-1 truncate text-sm" title={source}>{baseOf(source)}</span>
                  <span class="shrink-0 text-xs text-muted-foreground">{describe(source)}</span>
                  <button
                    type="button"
                    onclick={() => removeSource(source)}
                    aria-label="Remove {baseOf(source)}"
                    class="size-6 flex items-center justify-center rounded text-muted-foreground hover:text-foreground hover:bg-muted"
                  >
                    <XIcon strokeWidth={1.5} class="size-3.5" />
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="text-xs text-muted-foreground">
              Scrivener projects are chosen with “Choose files…”.
            </p>
          {/if}
        </div>

        <label class="grid gap-1.5">
          <span class="text-xs font-medium text-muted-foreground">Import into</span>
          <select
            bind:value={destination}
            class="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          >
            {#each locations as location (location.dir)}
              <option value={location.dir}>{location.label}</option>
            {/each}
          </select>
          <span class="text-xs text-muted-foreground">
            Projects and folders arrive as a new folder here, with their structure kept.
          </span>
        </label>

        {#if hasDocx}
          <label class="flex items-start gap-2 text-sm">
            <input type="checkbox" class="mt-1" bind:checked={splitDocx} />
            <span>
              Split Word documents at headings
              <span class="block text-xs text-muted-foreground">
                One writing per chapter, in a folder named after the document.
              </span>
            </span>
          </label>
        {/if}

        <Dialog.Footer class="">
          <Button variant="ghost" onclick={close} disabled={busy}>Cancel</Button>
          <Button onclick={runImport} disabled={busy || sources.length === 0}>
            {busy ? 'Importing…' : 'Import'}
          </Button>
        </Dialog.Footer>
      </div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
