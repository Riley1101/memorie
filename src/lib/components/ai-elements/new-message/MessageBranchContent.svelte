<script>
	import { cn } from "$lib/utils";
	import { getMessageBranchContext } from "./message-context.svelte.js";
	import { watch } from "runed";

	let { children, content, renderItem, class: className, ...restProps } = $props();

	const branchContext = getMessageBranchContext();

	// Compute items count based on whether we have snippets or content
	let itemsCount = $derived(children?.length ?? content?.length ?? 0);

	// Update branches count when items change
	watch(
		() => itemsCount,
		(newLength) => {
			if (branchContext.totalBranches !== newLength) {
				// Store the count, not the snippets themselves
				branchContext.setBranchCount(newLength);
			}
		}
	);
</script>

{#if children}
	<!-- Render snippets mode -->
	{#each children as branch, index (index)}
		<div
			class={cn(
				"grid gap-2 overflow-hidden [&>div]:pb-0",
				index === branchContext.currentBranch ? "block" : "hidden",
				className
			)}
			{...restProps}
		>
			{@render branch()}
		</div>
	{/each}
{:else if content && renderItem}
	<!-- Render content with renderItem mode -->
	{#each content as item, index (item.id)}
		<div
			class={cn(
				"grid gap-2 overflow-hidden [&>div]:pb-0",
				index === branchContext.currentBranch ? "block" : "hidden",
				className
			)}
			{...restProps}
		>
			{@render renderItem(item, index)}
		</div>
	{/each}
{/if}