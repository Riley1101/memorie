<script>
	import { cn } from "$lib/utils.js";
	let { class: className, children, ...restProps } = $props();
</script>

<span
	class={cn(
		"text-muted-foreground flex items-center gap-2 text-sm [&_svg:not([class*='size-'])]:size-4 [&_svg]:pointer-events-none",
		className
	)}
	{...restProps}
>
	{@render children?.()}
</span>