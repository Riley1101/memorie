<script>
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { MOD_LABEL } from '$lib/keyboard.svelte.js';
  import HouseIcon from '@lucide/svelte/icons/house';
  import SearchIcon from '@lucide/svelte/icons/search';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import SaveIcon from '@lucide/svelte/icons/save';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import UndoIcon from '@lucide/svelte/icons/undo-2';
  import RedoIcon from '@lucide/svelte/icons/redo-2';
  import TypeIcon from '@lucide/svelte/icons/type';

  const shortcuts = [
    { key: `${MOD_LABEL} + K`, description: 'Search writings', icon: SearchIcon },
    { key: `${MOD_LABEL} + L`, description: 'Toggle AI Chat', icon: SparklesIcon },
    { key: `${MOD_LABEL} + B`, description: 'Go Home / Exit', icon: HouseIcon },
    { key: `${MOD_LABEL} + U`, description: 'Toggle History', icon: HistoryIcon },
    { key: `${MOD_LABEL} + S`, description: 'Save writing (:w)', icon: SaveIcon },
    { key: `${MOD_LABEL} + Q`, description: 'Close writing (:q)' },
    { key: `${MOD_LABEL} + Z`, description: 'Undo version', icon: UndoIcon },
    { key: `${MOD_LABEL} + ⇧ + Z`, description: 'Redo version', icon: RedoIcon },
    { key: `${MOD_LABEL} + + / -`, description: 'Adjust Font Size', icon: TypeIcon },
    { key: ':', description: 'Enter Command Mode' },
    { key: 'i', description: 'Enter Insert Mode' },
    { key: 'Esc', description: 'Exit Mode' },
  ];
</script>

<Dialog.Root 
  open={appState.ui.isHelpModalOpen} 
  onOpenChange={(v) => appState.toggleHelpModal(v)}
>
  <Dialog.Content class="sm:max-w-[480px] p-0 overflow-hidden border-border/40 font-writer" portalProps={{}}>
    <Dialog.Header class="px-6 pt-6 pb-4 bg-muted/20 border-b border-border/20">
      <Dialog.Title class="text-xl font-normal tracking-tight">Keyboard Shortcuts</Dialog.Title>
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
                <Icon class="size-3.5 text-muted-foreground/40 group-hover:text-primary transition-colors" />
              {:else}
                <div class="size-3.5"></div>
              {/if}
              <span class="text-[13px] text-muted-foreground/80 group-hover:text-foreground transition-colors font-sans">{description}</span>
            </div>
            
            <div class="flex gap-1.5 items-center">
              {#each key.split(' / ') as part, i (part + i)}
                {#if i > 0}<span class="text-[10px] text-muted-foreground/30 font-sans italic">or</span>{/if}
                <div class="flex gap-1">
                  {#each part.split(' + ') as k, j (k + j)}
                    <kbd class="min-w-[20px] h-5 px-1.5 flex items-center justify-center rounded border border-border/60 bg-muted/10 text-muted-foreground/80 font-sans text-[10px] font-medium shadow-sm">
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
      <p class="text-[11px] text-muted-foreground/50 text-center font-sans tracking-wide">
        Press <kbd class="px-1 py-0.5 rounded border border-border/40 bg-background text-[9px]">Esc</kbd> to close at any time
      </p>
    </div>
  </Dialog.Content>
</Dialog.Root>
