<script>
	import { cn } from "$lib/utils";
	let { class: className, children, ...restProps } = $props();
</script>

<div class={cn("mt-4 flex w-full items-center justify-between gap-4", className)} {...restProps}>
	{@render children?.()}
</div>