<script>
  import { ScrollArea as ScrollAreaPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';

  /** @type {{ ref?: any, class?: any, orientation?: "vertical" | "horizontal", children?: any, [key: string]: any }} */
  let {
    ref = $bindable(null),
    class: className = undefined,
    /** @type {'vertical' | 'horizontal'} */
    orientation = 'vertical',
    children = undefined,
    ...restProps
  } = $props();
</script>

<ScrollAreaPrimitive.Scrollbar
  bind:ref
  data-slot="scroll-area-scrollbar"
  orientation={/** @type {any} */ (orientation)}
  class={cn(
    'flex touch-none select-none p-px transition-colors',
    orientation === 'vertical' && 'h-full w-2.5 border-l border-l-transparent',
    orientation === 'horizontal' && 'h-2.5 flex-col border-t border-t-transparent',
    className
  )}
  {...restProps}
>
  {@render children?.()}
  <ScrollAreaPrimitive.Thumb
    data-slot="scroll-area-thumb"
    class="bg-border relative flex-1 rounded-full"
  />
</ScrollAreaPrimitive.Scrollbar>
