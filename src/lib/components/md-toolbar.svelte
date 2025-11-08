<script>
  import BoldIcon from '@lucide/svelte/icons/bold';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import ItalicIcon from '@lucide/svelte/icons/italic';
  import UnderlineIcon from '@lucide/svelte/icons/underline';
  import * as Sidebar from '@/components/ui/sidebar/index.js';
  import * as ButtonGroup from '$lib/components/ui/button-group/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { Button } from '$lib/components/ui/button';
  import { editorState } from '$lib/runes/editor.svelte.js';
</script>

{#if editorState.editor}
  <div class="sticky flex items-center left-0 top-0 bg-background z-10 w-full p-2">
    <Sidebar.Trigger class="mr-auto" />
    <ButtonGroup.Root type="multiple" variant="outline">
      <Button
        aria-label="Toggle bold"
        onclick={() => editorState.editor.chain().focus().toggleBold().run()}
        variant="outline"
      >
        <BoldIcon class="h-4 w-4" />
      </Button>
      <Button
        variant="outline"
        aria-label="Toggle italic"
        onclick={() => editorState.editor.chain().focus().toggleItalic().run()}
      >
        <ItalicIcon class="h-4 w-4" />
      </Button>
      <Button
        variant="outline"
        aria-label="Toggle strikethrough"
        onclick={() => editorState.editor.chain().focus().toggleStrike().run()}
      >
        <UnderlineIcon class="h-4 w-4" />
      </Button>
      <Button
        variant="outline"
        aria-label="Toggle Ai chat"
        onclick={() => appState.toggleAiChat(!appState.ui.isChatOpen)}
      >
        <SparklesIcon class="h-4 w-4" />
      </Button>
    </ButtonGroup.Root>
  </div>
{/if}
