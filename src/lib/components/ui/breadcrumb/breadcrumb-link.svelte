<script>
  import { cn } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className = undefined,
    href = undefined,
    child,
    children = undefined,
    ...restProps
  } = $props();

  const attrs = $derived({
    'data-slot': 'breadcrumb-link',
    class: cn('hover:text-foreground transition-colors', className),
    href,
    ...restProps,
  });
</script>

{#if child}
  {@render child({ props: attrs })}
{:else}
  <a bind:this={ref} {...attrs}>
    {@render children?.()}
  </a>
{/if}
