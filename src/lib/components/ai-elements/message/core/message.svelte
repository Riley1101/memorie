<script>
	import { cn } from "$lib/utils";
	let { from, class: className, children, ...restProps } = $props();
	// indexing
</script>

<div
	class={cn(
		"group flex w-full max-w-[95%] flex-col gap-2",
		from === "user" ? "is-user ml-auto justify-end" : "is-assistant",
		className
	)}
	data-role={from}
	{...restProps}
>
	{@render children()}
</div>