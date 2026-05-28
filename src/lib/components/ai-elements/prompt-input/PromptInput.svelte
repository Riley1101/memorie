<script>
	import { cn } from "$lib/utils";
	import { watch } from "runed";
	import { onMount } from "svelte";
	import { AttachmentsContext, setAttachmentsContext } from "./attachments-context.svelte.js";

	let {
		class: className = undefined,
		accept = undefined,
		multiple = undefined,
		globalDrop = undefined,
		syncHiddenInput = undefined,
		clearOnSubmit = true,
		maxFiles = undefined,
		maxFileSize = undefined,
		onError = undefined,
		onSubmit,
		children = undefined,
		...props
	} = $props();

	let anchorRef = $state(null);
	let formRef = $state(null);
	let attachmentsContext = new AttachmentsContext(
		accept,
		multiple,
		maxFiles,
		maxFileSize,
		onError
	);

	// Find nearest form to scope drag & drop
	onMount(() => {
		let root = anchorRef?.closest("form");
		if (root instanceof HTMLFormElement) {
			formRef = root;
		}
	});

	// Attach drop handlers on nearest form
	watch(
		() => formRef,
		(formRef) => {
			if (!formRef) return;

			let onDragOver = (e) => {
				if (e.dataTransfer?.types?.includes("Files")) {
					e.preventDefault();
				}
			};

			let onDrop = (e) => {
				if (e.dataTransfer?.types?.includes("Files")) {
					e.preventDefault();
				}
				if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
					attachmentsContext.add(e.dataTransfer.files);
				}
			};

			formRef.addEventListener("dragover", onDragOver);
			formRef.addEventListener("drop", onDrop);

			return () => {
				formRef?.removeEventListener("dragover", onDragOver);
				formRef?.removeEventListener("drop", onDrop);
			};
		}
	);

	// Global drop handlers
	watch(
		() => globalDrop,
		(globalDrop) => {
			if (!globalDrop) return;

			let onDragOver = (e) => {
				if (e.dataTransfer?.types?.includes("Files")) {
					e.preventDefault();
				}
			};

			let onDrop = (e) => {
				if (e.dataTransfer?.types?.includes("Files")) {
					e.preventDefault();
				}
				if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
					attachmentsContext.add(e.dataTransfer.files);
				}
			};

			document.addEventListener("dragover", onDragOver);
			document.addEventListener("drop", onDrop);

			return () => {
				document.removeEventListener("dragover", onDragOver);
				document.removeEventListener("drop", onDrop);
			};
		}
	);

	// Note: File input cannot be programmatically set for security reasons
	// The syncHiddenInput prop is no longer functional
	watch(
		() => attachmentsContext.files,
		() => {
			if (syncHiddenInput && attachmentsContext.fileInputRef) {
				// Clear the input when items are cleared
				if (attachmentsContext.files.length === 0) {
					attachmentsContext.fileInputRef.value = "";
				}
			}
		}
	);

	let handleChange = (event) => {
		let target = event.currentTarget;
		if (target.files) {
			attachmentsContext.add(target.files);
		}
	};

	// Convert blob URLs to data URLs for proper serialization
	async function convertBlobUrlToDataUrl(url) {
		const response = await fetch(url);
		const blob = await response.blob();
		return new Promise((resolve, reject) => {
			const reader = new FileReader();
			reader.onloadend = () => resolve(reader.result);
			reader.onerror = reject;
			reader.readAsDataURL(blob);
		});
	}

	let handleSubmit = async (event) => {
		event.preventDefault();

		let form = event.currentTarget;
		let formData = new FormData(form);
		let text = (formData.get("message")) || "";

		// Convert blob URLs to data URLs asynchronously
		let filesPromises = attachmentsContext.files.map(async (item) => {
			if (item.url && item.url.startsWith("blob:")) {
				return {
					...item,
					url: await convertBlobUrlToDataUrl(item.url),
				};
			}
			return item;
		});

		try {
			let files = await Promise.all(filesPromises);
			let result = onSubmit({ text, files }, event);

			// Handle both sync and async onSubmit
			if (result && typeof result === "object" && "then" in result) {
				await result;
			}

			// Only clear if submission was successful
			if (clearOnSubmit) {
				attachmentsContext.clear();
				form.reset();
			}
		} catch (error) {
			// Don't clear on error - user may want to retry
			console.error("Submit failed:", error);
		}
	};

	setAttachmentsContext(attachmentsContext);
</script>

<span aria-hidden="true" class="hidden" bind:this={anchorRef}></span>
<input
	{accept}
	class="hidden"
	{multiple}
	onchange={handleChange}
	bind:this={attachmentsContext.fileInputRef}
	type="file"
/>
<form
	class={cn(
		"bg-background w-full divide-y overflow-hidden rounded-xl border shadow-sm",
		className
	)}
	onsubmit={handleSubmit}
	{...props}
>
	{#if children}
		{@render children()}
	{/if}
</form>