<script>
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
	import PromptInputButton from "./PromptInputButton.svelte";
	import PlusIcon from "./PlusIcon.svelte";

	let { class: className, children, ...props } = $props();
</script>

<DropdownMenu.Trigger>
	<PromptInputButton class={className} {...props}>
		{#if children}
			{@render children()}
		{:else}
			<PlusIcon class="size-4" />
		{/if}
	</PromptInputButton>
</DropdownMenu.Trigger>