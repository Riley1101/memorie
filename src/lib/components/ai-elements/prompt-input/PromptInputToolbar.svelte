<script>
	import { cn } from "$lib/utils";

	let { class: className, children, ...props } = $props();
</script>

<div class={cn("flex items-center justify-between p-1", className)} {...props}>
	{#if children}
		{@render children()}
	{/if}
</div>