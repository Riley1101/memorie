<script>
  import { cn } from '$lib/utils.js';

  /**
   * SidebarHeader Component
   * 
   * This component serves as the header section of a sidebar layout.
   * It accepts a reference, custom class names, and child elements to be rendered within the header.
   * 
   * Props:
   * - ref: A bindable reference to the header element (default: null).
   * - class?: Additional class names for styling the header.
   * - children: Child elements to be rendered inside the header.
   */
  let { ref = $bindable(null), class: className, children, ...restProps } = $props();
</script>

<div
  bind:this={ref}
  data-slot="sidebar-header"
  data-sidebar="header"
  class={cn('flex flex-col gap-2 p-2', className)}
  {...restProps}
>
  {@render children?.()}
</div>
