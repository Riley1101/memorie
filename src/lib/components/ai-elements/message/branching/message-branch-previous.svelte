<script>
	import { Button } from "$lib/components/ui/button/index.js";
	import { cn } from "$lib/utils";
	import ChevronLeft from "@lucide/svelte/icons/chevron-left";
	import { getMessageBranchContext } from "../context/message-context.svelte.js";

	let { class: className, children, ...restProps } = $props();

	const branchContext = getMessageBranchContext();

	const isDisabled = $derived(branchContext.totalBranches <= 1);
</script>

<Button
	aria-label="Previous branch"
	disabled={isDisabled}
	onclick={() => branchContext.goToPrevious()}
	size="icon"
	type="button"
	variant="ghost"
	class={cn("size-7", className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<ChevronLeft class="size-3.5" />
	{/if}
</Button>