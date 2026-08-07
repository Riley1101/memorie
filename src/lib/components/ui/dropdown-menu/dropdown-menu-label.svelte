<script>
	import { cn } from "$lib/utils.js";
	let {
		ref = $bindable(null),
		class: className,
		inset,
		children,
		...restProps
	} = $props();
</script>

<div
	bind:this={ref}
	data-slot="dropdown-menu-label"
	data-inset={inset}
	class={cn("px-[8px] py-[6px] text-[12px] font-medium text-muted-foreground data-[inset]:ps-8", className)}
	{...restProps}
>
	{@render children?.()}
</div>