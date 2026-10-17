<script>
	import { cn } from "$lib/utils.js";
	let {
		ref = $bindable(null),
		class: className = undefined,
		child = undefined,
		children = undefined,
		...restProps
	} = $props();

	/** @type {any} */
	const mergedProps = $derived({
		...restProps,
		class: cn("gap-2 rounded-lg border bg-muted px-2.5 text-sm font-medium [&_svg:not([class*='size-'])]:size-4 flex items-center [&_svg]:pointer-events-none", className),
		"data-slot": "button-group-text",
	});
</script>

{#if child}
	{@render child({ props: mergedProps })}
{:else}
	<div bind:this={ref} {...mergedProps}>
		{@render children?.()}
	</div>
{/if}