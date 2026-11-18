<script>
	import { cn } from "$lib/utils";
	let { class: className = undefined, children = undefined, ...restProps } = $props();
</script>

<div
	class={cn(
		"font-writer flex w-fit max-w-full min-w-0 flex-col gap-2 overflow-hidden text-base",
		"group-[.is-user]:bg-selection group-[.is-user]:text-foreground group-[.is-user]:ml-auto group-[.is-user]:rounded-group group-[.is-user]:px-3.5 group-[.is-user]:py-2.5",
		"group-[.is-assistant]:text-foreground/95",
		className
	)}
	{...restProps}
>
	{@render children()}
</div>