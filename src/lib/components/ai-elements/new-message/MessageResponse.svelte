<script>
	import { cn } from "$lib/utils";
	import { Streamdown } from "svelte-streamdown";
	import { mode } from "mode-watcher";
	let { content, class: className, ...restProps } = $props();
</script>

<div class={cn("size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0", className)}>
	<Streamdown
		{content}
		shikiTheme={mode.current === "dark" ? "github-dark-default" : "github-light-default"}
		shikiPreloadThemes={["github-dark-default", "github-light-default"]}
		baseTheme="shadcn"
		{...restProps}
	/>
</div>