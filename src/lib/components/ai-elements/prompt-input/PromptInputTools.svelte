<script>
	import { cn } from "$lib/utils";

	let { class: className, children, ...props } = $props();
</script>

<div
	class={cn("flex items-center gap-1", "[&_button:first-child]:rounded-bl-xl", className)}
	{...props}
>
	{#if children}
		{@render children()}
	{/if}
</div>