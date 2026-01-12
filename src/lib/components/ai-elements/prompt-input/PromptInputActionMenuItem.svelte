<script>
	import { cn } from "$lib/utils";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";

	let { class: className, children, onSelect, ...props } = $props();
</script>

<DropdownMenu.Item class={cn(className)} {onSelect} {...props}>
	{#if children}
		{@render children()}
	{/if}
</DropdownMenu.Item>