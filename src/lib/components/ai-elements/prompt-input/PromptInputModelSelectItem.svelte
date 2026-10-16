<script>
	import { cn } from "$lib/utils";
	import * as Select from "$lib/components/ui/select/index.js";

	let { class: className = undefined, value = undefined, children = undefined, ...props } = $props();
</script>

<Select.Item class={cn(className)} {value} {.../** @type {any} */ (props)}>
	{#if children}
		{@render children()}
	{/if}
</Select.Item>