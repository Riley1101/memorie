<script>
  import { scrollShadow } from '@/actions/scroll-shadow.js';

  /**
   * Wraps a ScrollArea and shows a fade at the top/bottom edges only while
   * there is more content to scroll to in that direction.
   * @type {{ class?: string, fadeSize?: string, children: import('svelte').Snippet }}
   */
  let { class: className = '', fadeSize = 'h-8', children } = $props();

  let canScrollUp = $state(false);
  let canScrollDown = $state(false);
</script>

<div
    class="relative {className}"
    use:scrollShadow={(s) => { canScrollUp = s.canScrollUp; canScrollDown = s.canScrollDown; }}
>
  {@render children()}
  <div class="pointer-events-none absolute top-0 inset-x-0 {fadeSize} bg-linear-to-b from-background to-transparent transition-opacity duration-200 {canScrollUp ? 'opacity-100' : 'opacity-0'}"></div>
  <div class="pointer-events-none absolute bottom-0 inset-x-0 {fadeSize} bg-linear-to-t from-background to-transparent transition-opacity duration-200 {canScrollDown ? 'opacity-100' : 'opacity-0'}"></div>
</div>
