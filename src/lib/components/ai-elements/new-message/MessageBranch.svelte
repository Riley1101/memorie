<script>
	import { cn } from "$lib/utils";
	import { MessageBranchClass, setMessageBranchContext } from "./message-context.svelte.js";
	let {
		defaultBranch = 0,
		onBranchChange,
		class: className,
		children,
		...restProps
	} = $props();

	// Create the branch context class
	const branchContext = new MessageBranchClass(defaultBranch);

	// Set up the context
	setMessageBranchContext(branchContext);

	// Watch for branch changes and call the callback
	// Using $derived to track changes without $effect
	let previousBranch = $state(defaultBranch);

	$effect.pre(() => {
		if (branchContext.currentBranch !== previousBranch) {
			previousBranch = branchContext.currentBranch;
			onBranchChange?.(branchContext.currentBranch);
		}
	});
</script>

<div class={cn("grid w-full gap-2 [&>div]:pb-0", className)} {...restProps}>
	{@render children()}
</div>