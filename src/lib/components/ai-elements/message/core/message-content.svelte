<script>
	import { cn } from "$lib/utils";
	let { class: className, children, ...restProps } = $props();
</script>

<div
	class={cn(
		"font-writer flex w-fit max-w-full min-w-0 flex-col gap-2 overflow-hidden text-base",
		"group-[.is-user]:bg-secondary group-[.is-user]:text-foreground group-[.is-user]:ml-auto group-[.is-user]:rounded-2xl group-[.is-user]:px-4 group-[.is-user]:py-2.5 group-[.is-user]:shadow-sm",
		"group-[.is-assistant]:text-foreground/95",
		className
	)}
	{...restProps}
>
	{@render children()}
</div>