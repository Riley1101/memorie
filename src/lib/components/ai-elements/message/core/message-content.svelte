<script>
	import { cn } from "$lib/utils";
	let { class: className, children, ...restProps } = $props();
</script>

<div
	class={cn(
		"flex w-fit max-w-full min-w-0 flex-col gap-2 overflow-hidden text-base",
		"group-[.is-user]:bg-secondary group-[.is-user]:text-foreground group-[.is-user]:ml-auto group-[.is-user]:rounded-lg group-[.is-user]:px-4 group-[.is-user]:py-3",
		"group-[.is-assistant]:text-foreground",
		className
	)}
	{...restProps}
>
	{@render children()}
</div>