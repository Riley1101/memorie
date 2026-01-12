<script>
	import { cn } from "$lib/utils";

	let { class: className, children, ...props } = $props();
</script>

<div class={cn(className, "flex flex-col")} {...props}>
	{#if children}
		{@render children()}
	{/if}
</div>