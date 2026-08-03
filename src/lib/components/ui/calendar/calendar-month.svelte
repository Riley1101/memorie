<script>
  import { cn } from '$lib/utils.js';
  let { ref = $bindable(null), class: className = undefined, children = undefined, ...restProps } = $props();
</script>

<div {...restProps} bind:this={ref} class={cn('flex flex-col', className)}>
  {@render children?.()}
</div>
