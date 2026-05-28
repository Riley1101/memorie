<script>
	import { cn } from "$lib/utils";
	import { watch } from "runed";
	import { getAttachmentsContext } from "./attachments-context.svelte.js";

	let { class: className = undefined, children = undefined, ...props } = $props();

	let attachments = getAttachmentsContext();
	let height = $state(0);
	let contentRef = $state(null);

	// Separate files and images for grouped rendering
	let nonImageFiles = $derived(
		attachments.files.filter((f) => !(f.mediaType?.startsWith("image/") && f.url))
	);

	let imageFiles = $derived(
		attachments.files.filter((f) => f.mediaType?.startsWith("image/") && f.url)
	);

	// Watch for resize changes using ResizeObserver
	watch(
		() => contentRef,
		(contentRef) => {
			if (!contentRef) return;

			let ro = new ResizeObserver(() => {
				if (contentRef) {
					height = contentRef.getBoundingClientRect().height;
				}
			});

			ro.observe(contentRef);
			height = contentRef.getBoundingClientRect().height;

			return () => ro.disconnect();
		}
	);

	let computedHeight = $derived.by(() => {
		return attachments.files.length ? height : 0;
	});
</script>

<div
	aria-live="polite"
	class={cn("overflow-hidden transition-[height] duration-200 ease-out", className)}
	style:height="{computedHeight}px"
	{...props}
>
	<div class="space-y-2 px-3 py-1" bind:this={contentRef}>
		<!-- Non-image files first -->
		{#if nonImageFiles.length > 0}
			<div class="flex flex-wrap gap-2">
				{#each nonImageFiles as file (file.id)}
					{#if children}
						{@render children(file)}
					{/if}
				{/each}
			</div>
		{/if}

		<!-- Images second -->
		{#if imageFiles.length > 0}
			<div class="flex flex-wrap gap-2">
				{#each imageFiles as file (file.id)}
					{#if children}
						{@render children(file)}
					{/if}
				{/each}
			</div>
		{/if}
	</div>
</div>