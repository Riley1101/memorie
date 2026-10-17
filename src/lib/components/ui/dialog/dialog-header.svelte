<script>
	import { cn } from "$lib/utils.js";
	let {
		ref = $bindable(null),
		class: className = undefined,
		children = undefined,
		...restProps
	} = $props();
</script>

<div
	bind:this={ref}
	data-slot="dialog-header"
	class={cn("gap-2 flex flex-col", className)}
	{...restProps}
>
	{@render children?.()}
</div>