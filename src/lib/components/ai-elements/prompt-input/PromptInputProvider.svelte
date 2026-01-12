<script>
	import { PromptInputController, setPromptInputProvider } from "./attachments-context.svelte.js";

	let { initialInput = "", accept, multiple = true, children } = $props();

	let controller = new PromptInputController(initialInput, accept, multiple);

	setPromptInputProvider(controller);
</script>

{#if children}
	{@render children()}
{/if}