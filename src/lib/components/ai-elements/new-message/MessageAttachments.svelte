<script>
	import { cn } from "$lib/utils";
	let { class: className, children, ...restProps } = $props();

	// Check if children is provided
	let hasChildren = $derived(!!children);
</script>

{#if hasChildren}
	<div class={cn("ml-auto flex w-fit flex-wrap items-start gap-2", className)} {...restProps}>
		{@render children?.()}
	</div>
{/if}