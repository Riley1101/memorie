<script>
	import { cn } from "$lib/utils";

	let { class: className = undefined, children = undefined, ...props } = $props();
</script>

<div class={cn(className, "flex flex-col")} {...props}>
	{#if children}
		{@render children()}
	{/if}
</div>