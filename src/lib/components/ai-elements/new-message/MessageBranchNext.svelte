<script>
	import { cn } from "$lib/utils";
	import { getMessageBranchContext } from "./message-context.svelte.js";
	import { Button } from "$lib/components/ui/button/index.js";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	let { class: className, children, ...restProps } = $props();

	const branchContext = getMessageBranchContext();

	let isDisabled = $derived(branchContext.totalBranches <= 1);
</script>

<Button
	aria-label="Next branch"
	disabled={isDisabled}
	onclick={() => branchContext.goToNext()}
	size="icon"
	type="button"
	variant="ghost"
	class={cn("size-7", className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<ChevronRight class="size-3.5" />
	{/if}
</Button>