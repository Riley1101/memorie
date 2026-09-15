<script>
  import CirclePlusIcon from '@lucide/svelte/icons/circle-plus';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import * as Command from '$lib/components/ui/command/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { fileManager, baseOf } from '$lib/runes/fs.svelte.js';
  import { formatFileName, formatDate, formatTime } from '@/utils';
  import { MOD_KEY } from '$lib/keyboard.svelte.js';
  import { startNewWriting, sanitizeTitle } from '$lib/new-writing.js';
  import { toast } from '$lib/toast.js';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  let search = $state('');

  // Where a writing created from the palette goes: the binder open on the
  // home screen, or the top level.
  let targetDir = $derived(appState.ui.activeBinder ?? '');
  let targetLabel = $derived(appState.ui.activeBinder ?? 'top level');

  /** Typed text as a usable title, if it doesn't already name a writing. */
  let creatableTitle = $derived.by(() => {
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
    goto(resolve(`/${encodeURIComponent(fileName)}`));
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
      goto(resolve(`/${encodeURIComponent(full)}`));
    } catch (e) {
      toast.error(`Could not create "${title}"`, e);
    }
  }

  function handleOpenSettings() {
    close();
    goto(resolve('/settings'));
  }
</script>

<Command.Dialog
  open={appState.ui.isCommandMenuOpen}
  onOpenChange={(v) => {
    appState.toggleCommandMenu(v);
    if (!v) search = '';
  }}
  title="Search Writings"
  description="Search writings by title, or type a new title to create one"
  portalProps={{}}
>
  <Command.Input
    bind:value={search}
    placeholder="Search or create…"
    class="border-border/20 bg-muted/10"
 />
  <Command.List class="p-2">
    <Command.Empty class="py-10 text-muted-foreground/60 font-sans">
      {#if creatableTitle}
        Press Enter to create “{creatableTitle}”.
      {:else}
        No results found.
      {/if}
    </Command.Empty>

    {#if creatableTitle}
      <Command.Group heading="Create" class="" value="">
        <Command.Item
          forceMount
          value={`create-named ${search}`}
          onSelect={handleCreateNamed}
          class="rounded-lg px-3 py-2.5 aria-selected:bg-muted/40"
        >
          <CirclePlusIcon strokeWidth={1.5} class="size-3.5 mr-2" />
          <span class="font-writer text-base">
            Create “{creatableTitle}”
            <span class="text-muted-foreground/60 text-sm">in {targetLabel}</span>
          </span>
          <Command.Shortcut class="">↵</Command.Shortcut>
        </Command.Item>
      </Command.Group>
    {/if}

    <Command.Group heading="Writings" class="" value="">
      {#each fileManager.files as file (file.name)}
        <Command.Item
            value={`${formatFileName(baseOf(file.name))} ${file.folder ?? ''}`}
            onSelect={() => handleSelect(file.name)}
            class="rounded-lg px-3 py-2.5 mb-1 last:mb-0 aria-selected:bg-muted/40"
        >
          <div class="flex flex-col gap-0.5 min-w-0 font-writer">
            <span class="text-base font-normal truncate">{formatFileName(baseOf(file.name))}</span>
            <div class="flex items-center gap-2 text-[0.6875rem] text-muted-foreground/60 font-mono">
              {#if file.folder}
                <span class="flex items-center gap-1">
                  <FolderIcon strokeWidth={1.5} class="size-2.5" />
                  {file.folder}
                </span>
                <span class="opacity-30">•</span>
              {/if}
              <span>{formatDate(file.last_modified)}</span>
              <span class="opacity-30">•</span>
              <span>{formatTime(file.last_modified)}</span>
            </div>
          </div>
        </Command.Item>
      {/each}
    </Command.Group>

    <Command.Group heading="Actions" class="" value="">
      <Command.Item value="new writing blank create" onSelect={handleCreateBlank} class="rounded-lg px-3 py-2.5 aria-selected:bg-muted/40">
        <CirclePlusIcon strokeWidth={1.5} class="size-3.5 mr-2" />
        <span>New writing in {targetLabel}</span>
        <Command.Shortcut class="">{MOD_KEY}N</Command.Shortcut>
      </Command.Item>
      <Command.Item value="new writing in choose location" onSelect={() => { close(); appState.toggleNewPicker(true); }} class="rounded-lg px-3 py-2.5 aria-selected:bg-muted/40">
        <FolderIcon strokeWidth={1.5} class="size-3.5 mr-2" />
        <span>New writing in…</span>
        <Command.Shortcut class="">{MOD_KEY}⇧N</Command.Shortcut>
      </Command.Item>
      <Command.Item value="settings preferences" onSelect={handleOpenSettings} class="rounded-lg px-3 py-2.5 aria-selected:bg-muted/40">
        <SettingsIcon strokeWidth={1.5} class="size-3.5 mr-2" />
        <span>Open settings</span>
      </Command.Item>
    </Command.Group>
  </Command.List>
</Command.Dialog>
