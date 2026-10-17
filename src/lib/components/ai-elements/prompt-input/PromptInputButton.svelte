<script>
	import { cn } from "$lib/utils";
	import { Button } from "$lib/components/ui/button";
	/** @type {{ variant?: "link" | "default" | "outline" | "secondary" | "ghost" | "destructive", class?: any, size?: "icon" | "default" | "xs" | "sm" | "lg" | "icon-xs" | "icon-sm" | "icon-lg", children?: any, [key: string]: any }} */
	let { variant = "ghost", class: className = undefined, size = undefined, children = undefined, ...props } = $props();

	let hasMultipleChildren = $derived.by(() => {
		// In Svelte, we can't easily count children like in React, so we'll default to checking if size is provided
		return size !== undefined;
	});

	let newSize = $derived.by(() => {
		return /** @type {"default" | "icon"} */ ((size ?? hasMultipleChildren) ? "default" : "icon");
	});
</script>

<Button
	class={cn(
		"shrink-0 gap-1.5 rounded-lg",
		variant === "ghost" && "text-muted-foreground",
		newSize === "default" && "px-3",
		className
	)}
	size={newSize}
	type="button"
	{variant}
	{...props}
>
	{#if children}
		{@render children()}
	{/if}
</Button>