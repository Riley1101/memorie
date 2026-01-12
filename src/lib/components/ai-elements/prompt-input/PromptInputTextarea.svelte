<script>
	import { cn } from "$lib/utils";
	import { Textarea } from "$lib/components/ui/textarea/index.js";
	import { getAttachmentsContext } from "./attachments-context.svelte.js";

	let {
		class: className,
		placeholder = "What would you like to know?",
		value = $bindable(""),
		onchange,
		...props
	} = $props();

	let attachments = getAttachmentsContext();

	let handleKeyDown = (e) => {
		if (e.key === "Enter") {
			// Don't submit if IME composition is in progress
			if (e.isComposing) {
				return;
			}

			if (e.shiftKey) {
				// Allow newline
				return;
			}

			// Submit on Enter (without Shift)
			e.preventDefault();
			let form = (e.currentTarget).form;
			if (form) {
				form.requestSubmit();
			}
		}
	};

	let handlePaste = (e) => {
		let items = e.clipboardData?.items;

		if (!items) {
			return;
		}

		let files = [];

		for (let item of items) {
			if (item.kind === "file") {
				let file = item.getAsFile();
				if (file) {
					files.push(file);
				}
			}
		}

		if (files.length > 0) {
			e.preventDefault();
			attachments.add(files);
		}
	};
</script>

<Textarea
	class={cn(
		"w-full resize-none rounded-none border-none p-3 shadow-none ring-0 outline-none",
		"field-sizing-content bg-transparent dark:bg-transparent",
		"max-h-48 min-h-16",
		"focus-visible:ring-0",
		className
	)}
	onpaste={handlePaste}
	name="message"
	{onchange}
	onkeydown={handleKeyDown}
	{placeholder}
	bind:value
	{...props}
/>