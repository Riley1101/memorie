<script>
	import { cn } from "$lib/utils";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";

	let { class: className, children, ...props } = $props();
</script>

<DropdownMenu.Content align="start" class={cn(className)} {...props}>
	{#if children}
		{@render children()}
	{/if}
</DropdownMenu.Content>