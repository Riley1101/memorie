<script>
	import { cn } from "$lib/utils";
	import * as Select from "$lib/components/ui/select/index.js";

	let { class: className, value, children, ...props } = $props();
</script>

<Select.Item class={cn(className)} {value} {...props}>
	{#if children}
		{@render children()}
	{/if}
</Select.Item>