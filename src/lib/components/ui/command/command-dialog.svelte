<script>
	import Command from "./command.svelte";
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	let {
		open = $bindable(false),
		ref = $bindable(null),
		value = $bindable(""),
		title = "Command Palette",
		description = "Search for a command to run",
		portalProps = undefined,
		children = undefined,
		...restProps
	} = $props();
</script>

<Dialog.Root bind:open {...restProps}>
	<Dialog.Header class="sr-only">
		<Dialog.Title>{title}</Dialog.Title>
		<Dialog.Description>{description}</Dialog.Description>
	</Dialog.Header>
	<!-- Spotlight-style: a wide glass panel high in the window, a large field,
	     36px rows and the selected row in the selection tint. -->
	<Dialog.Content
		class="overflow-hidden p-0 gap-0 sm:max-w-155 top-30 translate-y-0 rounded-[18px]! shadow-[inset_0_1px_0_var(--glass-rim),0_30px_70px_-20px_rgb(0_0_0/0.45)]!"
		showCloseButton={false}
		{portalProps}
	>
		<Command
			class="bg-transparent **:data-[slot=command-input-wrapper]:h-14 **:data-[slot=command-input-wrapper]:px-4.5 **:data-[slot=command-input-wrapper]:gap-3 [&_[data-command-input-wrapper]_svg]:size-5 [&_[data-command-input]]:h-14 [&_[data-command-input]]:text-[20px] [&_[data-command-group]]:px-0 [&_[data-command-item]]:h-9 [&_[data-command-item]]:px-3 [&_[data-command-item]]:py-0 [&_[data-command-item]]:gap-2.5 [&_[data-command-item]]:rounded-[9px] [&_[data-command-item]_svg]:size-4 [&_[data-command-item]_svg]:text-primary"
			{...restProps}
			bind:value
			bind:ref
			{children}
		/>
	</Dialog.Content>
</Dialog.Root>