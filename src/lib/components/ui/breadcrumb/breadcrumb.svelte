<script>
  let { ref = $bindable(null), class: className = undefined, children = undefined, ...restProps } = $props();
</script>

<nav
  bind:this={ref}
  data-slot="breadcrumb"
  class={className}
  aria-label="breadcrumb"
  {...restProps}
>
  {@render children?.()}
</nav>
