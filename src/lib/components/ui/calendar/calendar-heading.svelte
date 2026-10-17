<script>
  import { Calendar as CalendarPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';

  let { ref = $bindable(null), class: className = undefined, ...restProps } = $props();
</script>

<CalendarPrimitive.Heading
  bind:ref
  class={cn('px-(--cell-size) text-sm font-medium', className)}
  {...restProps}
/>
