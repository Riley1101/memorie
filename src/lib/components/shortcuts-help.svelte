<script>
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { MOD_LABEL } from '$lib/keyboard.svelte.js';

  const shortcuts = [
    { key: `${MOD_LABEL} + K`, description: 'Search documents by title' },
    { key: `${MOD_LABEL} + L`, description: 'Toggle AI Chat' },
    { key: `${MOD_LABEL} + B`, description: 'Toggle Sidebar' },
    { key: `${MOD_LABEL} + U`, description: 'Toggle History Sidebar' },
    { key: `${MOD_LABEL} + S`, description: 'Save current document (Vim :w)' },
    { key: `${MOD_LABEL} + W`, description: 'Save and Close document (Vim :wq)' },
    { key: `${MOD_LABEL} + Q`, description: 'Close document (Vim :q)' },
    { key: `${MOD_LABEL} + H`, description: 'Show this help modal' },
    { key: `${MOD_LABEL} + Z`, description: 'Undo (Back one version)' },
    { key: `${MOD_LABEL} + ⇧ + Z / ${MOD_LABEL} + Y`, description: 'Redo (Forward to latest branch)' },
    { key: ':', description: 'Enter Command Mode' },
    { key: 'i', description: 'Enter Insert Mode (Edit document)' },
    { key: 'Esc', description: 'Exit Insert/Command Mode' },
  ];
</script>

<Dialog.Root 
  open={appState.ui.isHelpModalOpen} 
  onOpenChange={(v) => appState.toggleHelpModal(v)}
>
  <Dialog.Content class="sm:max-w-[500px]" portalProps={{}}>
    <Dialog.Header class="sr-only">
      <Dialog.Title class="">Keyboard Shortcuts</Dialog.Title>
      <Dialog.Description class="">
        List of all available shortcuts and commands in Memorie.
      </Dialog.Description>
    </Dialog.Header>

    <div class="grid gap-2 py-2">
      <div class="space-y-1.5">
        {#each shortcuts as { key, description }}
          <div class="flex items-center justify-between text-[11px] font-mono hover:bg-accent/50 p-1 px-2 rounded-sm transition-colors group">
            <span class="text-muted-foreground group-hover:text-foreground transition-colors">{description}</span>
            <div class="flex gap-1 items-center">
              {#each key.split(' / ') as part, i}
                {#if i > 0}<span class="text-[9px] opacity-40">or</span>{/if}
                <div class="flex gap-0.5">
                  {#each part.split(' + ') as k}
                    <kbd class="pointer-events-none inline-flex h-4 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[9px] font-medium text-muted-foreground">
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
  </Dialog.Content>
</Dialog.Root>
