<script>
  import { cn } from '$lib/utils.js';
  import { Popover as PopoverPrimitive } from 'bits-ui';

  /** @type {{ ref?: any, class?: any, sideOffset?: any, align?: "center" | "end" | "start", portalProps?: any, [key: string]: any }} */
  let {
    ref = $bindable(null),
    class: className = undefined,
    sideOffset = 4,
    /** @type {'center' | 'end' | 'start'} */
    align = 'center',
    portalProps = undefined,
    ...restProps
  } = $props();
</script>

<PopoverPrimitive.Portal {...portalProps}>
  <PopoverPrimitive.Content
    bind:ref
    data-slot="popover-content"
    {sideOffset}
    align={/** @type {any} */ (align)}
    class={cn(
      'bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2 origin-(--bits-popover-content-transform-origin) outline-hidden z-50 w-72 rounded-xl ring-1 ring-foreground/10 p-4 shadow-lg',
      className
    )}
    {...restProps}
  />
</PopoverPrimitive.Portal>
