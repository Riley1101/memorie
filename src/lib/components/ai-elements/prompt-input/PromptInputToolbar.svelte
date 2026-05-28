<script>
	import { cn } from "$lib/utils";

	let { class: className = undefined, children = undefined, ...props } = $props();
</script>

<div class={cn("flex items-center justify-between p-1", className)} {...props}>
	{#if children}
		{@render children()}
	{/if}
</div>