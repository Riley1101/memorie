<script>
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { MOD_LABEL } from '$lib/keyboard.svelte.js';
  import HouseIcon from '@lucide/svelte/icons/house';
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

  let shortcuts = $derived(
    [
      { key: `${MOD_LABEL} + K`, description: 'Search writings', icon: SearchIcon },
      { key: `${MOD_LABEL} + ⇧ + F`, description: 'Find in all writings', icon: TextSearchIcon },
      { key: `${MOD_LABEL} + ⇧ + E`, description: 'Export', icon: FileOutputIcon },
      { key: `${MOD_LABEL} + N`, description: 'New writing here', icon: PlusIcon },
      { key: `${MOD_LABEL} + ⇧ + N`, description: 'New writing in…', icon: PlusIcon },
      configManager.config?.ai_enabled
        ? { key: `${MOD_LABEL} + L`, description: 'Toggle AI chat', icon: SparklesIcon }
        : null,
      { key: `${MOD_LABEL} + ⇧ + H`, description: 'Go home', icon: HouseIcon },
      { key: `${MOD_LABEL} + U`, description: 'Toggle history', icon: HistoryIcon },
      { key: `${MOD_LABEL} + ⇧ + O`, description: 'Toggle table of contents', icon: TableOfContentsIcon },
      { key: `${MOD_LABEL} + .`, description: 'Toggle focus mode', icon: FocusIcon },
      { key: `${MOD_LABEL} + S`, description: 'Save writing', icon: SaveIcon },
      { key: `${MOD_LABEL} + Z`, description: 'Undo typing', icon: UndoIcon },
      { key: `${MOD_LABEL} + ⇧ + Z`, description: 'Redo typing', icon: RedoIcon },
      { key: `${MOD_LABEL} + ⌥ + Z`, description: 'Previous saved version', icon: UndoIcon },
      { key: `${MOD_LABEL} + ⌥ + ⇧ + Z`, description: 'Next saved version', icon: RedoIcon },
      { key: `${MOD_LABEL} + + / -`, description: 'Adjust font size', icon: TypeIcon },
      { key: `${MOD_LABEL} + ⇧ + P`, description: 'Command mode' },
      { key: `${MOD_LABEL} + /`, description: 'This help' },
      { key: 'F2', description: 'Rename selected writing' },
    ].filter(Boolean)
  );
</script>

<Dialog.Root 
  open={appState.ui.isHelpModalOpen} 
  onOpenChange={(v) => appState.toggleHelpModal(v)}
>
  <Dialog.Content class="sm:max-w-[480px] p-0 overflow-hidden border-border/40" portalProps={{}}>
    <Dialog.Header class="px-6 pt-6 pb-4 bg-muted/20 border-b border-border/20">
      <Dialog.Title class="text-xl font-normal font-writer tracking-tight">Keyboard Shortcuts</Dialog.Title>
      <Dialog.Description class="text-sm text-muted-foreground/60">
        Master your workflow with Memorie's quick commands.
      </Dialog.Description>
    </Dialog.Header>

    <div class="px-2 py-4 max-h-[70vh] overflow-y-auto">
      <div class="space-y-0.5">
        {#each shortcuts as { key, description, icon } (description)}
          <div class="flex items-center justify-between px-4 py-2 hover:bg-muted/30 rounded-lg transition-all group">
            <div class="flex items-center gap-3">
              {#if icon}
                {@const Icon = icon}
                <Icon strokeWidth={1.5} class="size-3.5 text-muted-foreground/40 group-hover:text-primary transition-colors" />
              {:else}
                <div class="size-3.5"></div>
              {/if}
              <span class="text-[0.8125rem] text-muted-foreground/80 group-hover:text-foreground transition-colors font-sans">{description}</span>
            </div>
            
            <div class="flex gap-1.5 items-center">
              {#each key.split(' / ') as part, i (part + i)}
                {#if i > 0}<span class="text-[0.625rem] text-muted-foreground/30 font-sans italic">or</span>{/if}
                <div class="flex gap-1">
                  {#each part.split(' + ') as k, j (k + j)}
                    <kbd class="min-w-[1.25rem] h-5 px-1.5 flex items-center justify-center rounded border border-border/60 bg-muted/10 text-muted-foreground/80 font-sans text-[0.625rem] font-medium shadow-sm">
                      {k}
                    </kbd>
                  {/each}
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </div>
    
    <div class="px-6 py-4 bg-muted/10 border-t border-border/10">
      <p class="text-[0.6875rem] text-muted-foreground/50 text-center font-sans tracking-wide">
        Press <kbd class="px-1 py-0.5 rounded border border-border/40 bg-background text-[0.5625rem]">Esc</kbd> to close at any time
      </p>
    </div>
  </Dialog.Content>
</Dialog.Root>
