<script>
	import { cn } from "$lib/utils";
	import * as Select from "$lib/components/ui/select/index.js";

	let { class: className = undefined, children = undefined, ...props } = $props();
</script>

<Select.Content class={cn(className)} {...props}>
	{#if children}
		{@render children()}
	{/if}
</Select.Content>