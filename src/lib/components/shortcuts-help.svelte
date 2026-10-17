<script>
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { MOD_KEY } from '$lib/keyboard.svelte.js';
  import { page } from '$app/state';
  import HouseIcon from '@lucide/svelte/icons/house';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import SearchIcon from '@lucide/svelte/icons/search';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import SaveIcon from '@lucide/svelte/icons/save';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import TableOfContentsIcon from '@lucide/svelte/icons/table-of-contents';
  import UndoIcon from '@lucide/svelte/icons/undo-2';
  import RedoIcon from '@lucide/svelte/icons/redo-2';
  import TypeIcon from '@lucide/svelte/icons/type';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import FocusIcon from '@lucide/svelte/icons/focus';
  import TextSearchIcon from '@lucide/svelte/icons/text-search';
  import FileOutputIcon from '@lucide/svelte/icons/file-output';
  import LinkIcon from '@lucide/svelte/icons/link';
  import TerminalIcon from '@lucide/svelte/icons/square-terminal';
  import KeyboardIcon from '@lucide/svelte/icons/keyboard';
  import PencilIcon from '@lucide/svelte/icons/pencil';

  /**
   * Most shortcuts are the editor's: they are bound by the command bar, which
   * only exists while a writing is open. Listing them on the home screen as if
   * they worked everywhere is what made them feel broken.
   */
  let inEditor = $derived(Boolean(page.params.lexical));

  let shortcuts = $derived(
    [
      { key: `${MOD_KEY} + K`, description: 'Search writings', icon: SearchIcon },
      { key: `${MOD_KEY} + K`, description: 'Link selected text', editorOnly: true, icon: LinkIcon },
      { key: `${MOD_KEY} + Click`, description: 'Open link', editorOnly: true, icon: LinkIcon },
      { key: `${MOD_KEY} + ⇧ + F`, description: 'Find in all writings', icon: TextSearchIcon },
      { key: `${MOD_KEY} + ⇧ + E`, description: 'Export', editorOnly: true, icon: FileOutputIcon },
      { key: `${MOD_KEY} + N`, description: 'New writing here', icon: PlusIcon },
      { key: `${MOD_KEY} + ⇧ + N`, description: 'New writing in…', icon: PlusIcon },
      configManager.config?.ai_enabled
        ? { key: `${MOD_KEY} + L`, description: 'Toggle AI chat', editorOnly: true, icon: SparklesIcon }
        : null,
      { key: `${MOD_KEY} + B`, description: 'Go back (bold while typing)', icon: ArrowLeftIcon },
      { key: `${MOD_KEY} + ⇧ + H`, description: 'Go home', editorOnly: true, icon: HouseIcon },
      { key: `${MOD_KEY} + U`, description: 'Toggle history', editorOnly: true, icon: HistoryIcon },
      { key: `${MOD_KEY} + ⇧ + O`, description: 'Toggle table of contents', editorOnly: true, icon: TableOfContentsIcon },
      { key: `${MOD_KEY} + .`, description: 'Toggle focus mode', editorOnly: true, icon: FocusIcon },
      { key: `${MOD_KEY} + S`, description: 'Save writing', editorOnly: true, icon: SaveIcon },
      { key: `${MOD_KEY} + Z`, description: 'Undo typing', editorOnly: true, icon: UndoIcon },
      { key: `${MOD_KEY} + ⇧ + Z`, description: 'Redo typing', editorOnly: true, icon: RedoIcon },
      { key: `${MOD_KEY} + ⌥ + Z`, description: 'Previous saved version', editorOnly: true, icon: UndoIcon },
      { key: `${MOD_KEY} + ⌥ + ⇧ + Z`, description: 'Next saved version', editorOnly: true, icon: RedoIcon },
      { key: `${MOD_KEY} + + / -`, description: 'Adjust font size', icon: TypeIcon },
      { key: `${MOD_KEY} + ⇧ + P`, description: 'Command mode', icon: TerminalIcon },
      { key: `${MOD_KEY} + /`, description: 'This help', icon: KeyboardIcon },
      { key: 'F2', description: 'Rename selected writing', icon: PencilIcon },
    ].filter((s) => s && (inEditor || !s.editorOnly))
  );
</script>

<Dialog.Root
  open={appState.ui.isHelpModalOpen}
  onOpenChange={(v) => appState.toggleHelpModal(v)}
>
  <Dialog.Content class="sm:max-w-[480px] p-0 overflow-hidden" portalProps={{}}>
    <Dialog.Header class="px-6 pt-6 pb-2">
      <Dialog.Title class="text-xl font-normal font-writer tracking-tight">Keyboard shortcuts</Dialog.Title>
      <Dialog.Description class="text-sm text-muted-foreground">
        {inEditor ? 'Everything you can do from the keyboard while writing.' : 'These work from here. Open a writing to see the editor’s shortcuts too.'}
      </Dialog.Description>
    </Dialog.Header>

    <ul class="px-2 pb-4 max-h-[70vh] overflow-y-auto">
      {#each shortcuts as { key, description, icon } (description)}
        {@const Icon = icon}
        <li class="flex items-center justify-between gap-4 px-4 py-2">
          <div class="flex items-center gap-3">
            <Icon strokeWidth={1.5} class="size-3.5 text-muted-foreground shrink-0" />
            <span class="text-[0.8125rem] text-foreground">{description}</span>
          </div>

          <div class="flex gap-1.5 items-center shrink-0">
            {#each key.split(' / ') as part, i (part + i)}
              {#if i > 0}<span class="text-[0.6875rem] text-muted-foreground">or</span>{/if}
              <div class="flex gap-1">
                {#each part.split(' + ') as k, j (k + j)}
                  <kbd class="min-w-[1.375rem] h-5.5 px-1.5 flex items-center justify-center rounded bg-muted text-muted-foreground font-sans text-[0.6875rem] font-medium">
                    {k}
                  </kbd>
                {/each}
              </div>
            {/each}
          </div>
        </li>
      {/each}
    </ul>
  </Dialog.Content>
</Dialog.Root>
