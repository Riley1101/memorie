<script>
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { appState } from '$lib/runes/app.svelte.js';

  const shortcuts = [
    { key: '⌘ + K', description: 'Search documents by title' },
    { key: '⌘ + L', description: 'Toggle AI Chat' },
    { key: '⌘ + B', description: 'Toggle Sidebar' },
    { key: '⌘ + U', description: 'Toggle History Sidebar' },
    { key: '⌘ + S', description: 'Save current document (Vim :w)' },
    { key: '⌘ + W', description: 'Save and Close document (Vim :wq)' },
    { key: '⌘ + Q', description: 'Close document (Vim :q)' },
    { key: '⌘ + H', description: 'Show this help modal' },
    { key: '⌃ + Z', description: 'Undo (Back one version)' },
    { key: '⌃ + ⇧ + Z / ⌃ + Y', description: 'Redo (Forward to latest branch)' },
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
    <Dialog.Header class="">
      <Dialog.Title class="">Keyboard Shortcuts</Dialog.Title>
      <Dialog.Description class="">
        List of all available shortcuts and commands in Memorie.
      </Dialog.Description>
    </Dialog.Header>

    <div class="grid gap-4 py-4">
      <div class="border rounded-md overflow-hidden font-mono text-sm">
        <div class="grid grid-cols-[1fr_2fr] bg-muted/50 border-b p-2 font-bold">
          <span>Shortcut</span>
          <span>Action</span>
        </div>
        {#each shortcuts as { key, description }}
          <div class="grid grid-cols-[1fr_2fr] p-2 border-b last:border-0 hover:bg-accent/50 transition-colors">
            <span class="text-primary font-bold">{key}</span>
            <span class="text-muted-foreground">{description}</span>
          </div>
        {/each}
      </div>
    </div>
  </Dialog.Content>
</Dialog.Root>
