<script module>
  import { getContext, setContext } from 'svelte';
  export function setToggleGroupCtx(props) {
    setContext('toggleGroup', props);
  }

  export function getToggleGroupCtx() {
    return getContext('toggleGroup');
  }
</script>

<script>
  import { ToggleGroup as ToggleGroupPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    value = $bindable(),
    class: className = undefined,
    size = 'default',
    variant = 'default',
    ...restProps
  } = $props();

  // Getters, so a later change to `variant` or `size` reaches the items
  // instead of freezing at the value they had when the group mounted.
  setToggleGroupCtx({
    get variant() {
      return variant;
    },
    get size() {
      return size;
    },
  });
</script>

<!--
Discriminated Unions + Destructing (required for bindable) do not
get along, so we shut typescript up by casting `value` to `never`.
-->
<ToggleGroupPrimitive.Root
  bind:value
  bind:ref
  data-slot="toggle-group"
  data-variant={variant}
  data-size={size}
  class={cn(
    'group/toggle-group data-[variant=outline]:shadow-xs flex w-fit items-center rounded-md',
    className
  )}
  {.../** @type {any} */ (restProps)}
/>
