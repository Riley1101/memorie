<script>
	import { cn } from "$lib/utils";
	let { class: className = undefined, children = undefined, ...restProps } = $props();
</script>

<div class={cn("flex items-center gap-1", className)} {...restProps}>
	{@render children?.()}
</div>