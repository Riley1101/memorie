<script>
	import { cn } from "$lib/utils";
	import { getMessageBranchContext } from "./message-context.svelte.js";
	import { Button } from "$lib/components/ui/button/index.js";
	import ChevronLeft from "@lucide/svelte/icons/chevron-left";
	let { class: className, children, ...restProps } = $props();

	const branchContext = getMessageBranchContext();

	let isDisabled = $derived(branchContext.totalBranches <= 1);
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