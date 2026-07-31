<script>
	import { cn } from "$lib/utils";
	import {
		MessageBranchController,
		setMessageBranchContext,
	} from "../context/message-context.svelte.js";
	let {
		defaultBranch = 0,
		onBranchChange,
		class: className,
		children,
		...restProps
	} = $props();

	const branchContext = new MessageBranchController();
	setMessageBranchContext(branchContext);

	let initialized = $state(false);
	let previousBranch = $state(null);

	$effect.pre(() => {
		if (!initialized) {
			branchContext.setCurrentBranch(defaultBranch);
			previousBranch = branchContext.currentBranch;
			initialized = true;
		}
	});

	$effect(() => {
		const currentBranch = branchContext.currentBranch;

		if (previousBranch === null) {
			previousBranch = currentBranch;
			return;
		}

		if (currentBranch !== previousBranch) {
			previousBranch = currentBranch;
			onBranchChange?.(currentBranch);
		}
	});
</script>

<div class={cn("grid w-full gap-2 [&>div]:pb-0", className)} {...restProps}>
	{@render children()}
</div>