<script>
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";

	let { open = $bindable(false), children = undefined, ...props } = $props();
</script>

<DropdownMenu.Root bind:open {...props}>
	{#if children}
		{@render children()}
	{/if}
</DropdownMenu.Root>