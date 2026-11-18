<script>
  import { memoryManager } from '@/runes/memory.svelte.js';

  /** @type {import("./$types").LayoutProps} */
  let { data, children } = $props();

  let { content, fileName } = $derived(data);

  $effect(() => {
    memoryManager.setContext(content, fileName);
  });
</script>

{@render children()}
