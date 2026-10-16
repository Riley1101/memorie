<script>
	import { PromptInputController, setPromptInputProvider } from "./attachments-context.svelte.js";

	let { initialInput = "", accept, multiple = true, children = undefined } = $props();

	// Starting input and attachment config, read once at construction.
	// svelte-ignore state_referenced_locally
	let controller = new PromptInputController(initialInput, accept, multiple);

	setPromptInputProvider(controller);
</script>

{#if children}
	{@render children()}
{/if}