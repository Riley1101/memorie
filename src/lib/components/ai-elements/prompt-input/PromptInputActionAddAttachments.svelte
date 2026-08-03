<script>
  import { getAttachmentsContext } from './attachments-context.svelte.js';
  import PromptInputActionMenuItem from './PromptInputActionMenuItem.svelte';
  import ImageIcon from './ImageIcon.svelte';
  let { label = 'Add photos or files', class: className = undefined, ...props } = $props();

  let attachments = getAttachmentsContext();

  let handleSelect = (e) => {
    e.preventDefault();
    attachments.openFileDialog();
  };
</script>

<PromptInputActionMenuItem class={className} onSelect={handleSelect} {...props}>
  <ImageIcon class="mr-2 size-4" />
  {label}
</PromptInputActionMenuItem>
