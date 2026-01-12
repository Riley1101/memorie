<script>
	import { Streamdown } from "svelte-streamdown";
	import { cn } from "$lib/utils";
	import { mode } from "mode-watcher";

	let { class: className, ...restProps } = $props();
</script>

<Streamdown
	class={cn("size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0", className)}
	shikiTheme={mode.current === "dark" ? "github-dark-default" : "github-light-default"}
	shikiPreloadThemes={["github-dark-default", "github-light-default"]}
	baseTheme="shadcn"
	{...restProps}
/>