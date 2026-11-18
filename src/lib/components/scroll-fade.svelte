<script>
  import { scrollShadow } from '@/actions/scroll-shadow.js';

  /**
   * Wraps a ScrollArea and fades the top/bottom edges only while there is more
   * content to scroll to in that direction. A mask rather than a painted
   * gradient, so it fades into whatever is behind: paper, glass or the frame.
   * @type {{ class?: string, fade?: number, children: import('svelte').Snippet }}
   */
  let { class: className = '', fade = 32, children } = $props();

  let canScrollUp = $state(false);
  let canScrollDown = $state(false);

  const mask = $derived(
    `linear-gradient(to bottom, transparent 0, #000 ${canScrollUp ? fade : 0}px, #000 calc(100% - ${canScrollDown ? fade : 0}px), transparent 100%)`
  );
</script>

<div
  class="relative {className}"
  style:mask-image={mask}
  style:-webkit-mask-image={mask}
  use:scrollShadow={(s) => {
    canScrollUp = s.canScrollUp;
    canScrollDown = s.canScrollDown;
  }}
>
  {@render children()}
</div>
