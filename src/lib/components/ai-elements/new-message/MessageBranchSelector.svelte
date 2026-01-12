<script>
	import { cn } from "$lib/utils";
	import { getMessageBranchContext } from "./message-context.svelte.js";
	import * as ButtonGroup from "$lib/components/ui/button-group/index.js";
	let { from, class: className, children, ...restProps } = $props();

	const branchContext = getMessageBranchContext();

	// Don't render if there's only one branch
	let shouldRender = $derived(branchContext.totalBranches > 1);
</script>

{#if shouldRender}
	<ButtonGroup.Root
		class={cn(
			"[&>*:not(:first-child)]:rounded-l-md [&>*:not(:last-child)]:rounded-r-md",
			className
		)}
		orientation="horizontal"
		{...restProps}
	>
		{@render children?.()}
	</ButtonGroup.Root>
{/if}