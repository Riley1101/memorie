<script module>
  import { tv } from 'tailwind-variants';
  export const sheetVariants = tv({
    base: 'bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out fixed z-50 flex flex-col gap-4 shadow-lg transition ease-in-out data-[state=closed]:duration-150 data-[state=open]:duration-200',
    variants: {
      side: {
        top: 'data-[state=closed]:slide-out-to-top data-[state=open]:slide-in-from-top inset-x-2 top-2 h-auto',
        bottom:
          'data-[state=closed]:slide-out-to-bottom data-[state=open]:slide-in-from-bottom inset-x-2 bottom-2 h-auto',
        left: 'data-[state=closed]:slide-out-to-left data-[state=open]:slide-in-from-left inset-y-2 left-2 w-3/4 sm:max-w-sm',
        right:
          'data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right inset-y-2 right-2 w-3/4 sm:max-w-sm',
      },
    },
    defaultVariants: {
      side: 'right',
    },
  });
</script>

<script>
  import { Dialog as SheetPrimitive } from 'bits-ui';
  import XIcon from '@lucide/svelte/icons/x';
  import SheetOverlay from './sheet-overlay.svelte';
  import { cn } from '$lib/utils.js';

  /** @type {{ ref?: any, class?: any, side?: "top" | "right" | "bottom" | "left", portalProps?: any, children?: any, [key: string]: any }} */
  let {
    ref = $bindable(null),
    class: className = undefined,
    /** @type {'top' | 'right' | 'bottom' | 'left'} */
    side = 'right',
    portalProps = undefined,
    children = undefined,
    ...restProps
  } = $props();
</script>

<SheetPrimitive.Portal {...portalProps}>
  <SheetOverlay class={undefined} />
  <SheetPrimitive.Content
    bind:ref
    data-slot="sheet-content"
    class={cn(sheetVariants(/** @type {any} */ ({ side })), className)}
    {...restProps}
  >
    {@render children?.()}
    <SheetPrimitive.Close
      class="ring-offset-background focus-visible:ring-ring rounded-xs focus-visible:outline-hidden absolute right-4 top-4 opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-offset-2 disabled:pointer-events-none"
    >
      <XIcon class="size-4" />
      <span class="sr-only">Close</span>
    </SheetPrimitive.Close>
  </SheetPrimitive.Content>
</SheetPrimitive.Portal>
