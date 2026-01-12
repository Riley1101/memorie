<script>
	import * as Select from "$lib/components/ui/select/index.js";

	let { value = $bindable(""), onValueChange, children, ...props } = $props();
</script>

<Select.Root bind:value {onValueChange} type="single" {...props}>
	{#if children}
		{@render children()}
	{/if}
</Select.Root>