<script>
  import { Calendar as CalendarPrimitive } from 'bits-ui';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import { cn } from '$lib/utils.js';

  /** @type {{ ref?: any, class?: any, children?: any, variant?: "link" | "default" | "outline" | "secondary" | "ghost" | "destructive", [key: string]: any }} */
  let {
    ref = $bindable(null),
    class: className = undefined,
    children = undefined,
    variant = 'ghost',
    ...restProps
  } = $props();
</script>

{#snippet Fallback()}
  <ChevronRightIcon class="size-4" />
{/snippet}

<CalendarPrimitive.NextButton
  bind:ref
  class={cn(
    buttonVariants({ variant }),
    'size-(--cell-size) select-none bg-transparent p-0 disabled:opacity-50 rtl:rotate-180',
    className
  )}
  children={children || Fallback}
  {...restProps}
/>
