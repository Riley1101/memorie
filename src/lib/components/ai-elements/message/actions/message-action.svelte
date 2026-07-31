<script>
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Tooltip from "$lib/components/ui/tooltip/index.js";
	import { cn } from "$lib/utils";
	let {
		tooltip,
		label,
		variant = "ghost",
		size = "icon",
		class: className,
		children,
		...restProps
	} = $props();

	const srOnlyLabel = $derived(label || tooltip);
</script>

{#if tooltip}
	<Tooltip.Provider>
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						{...restProps}
						{size}
						type="button"
						{variant}
						class={cn("size-7", className)}
					>
						{@render children?.()}
						{#if srOnlyLabel}
							<span class="sr-only">{srOnlyLabel}</span>
						{/if}
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content>
				<p>{tooltip}</p>
			</Tooltip.Content>
		</Tooltip.Root>
	</Tooltip.Provider>
{:else}
	<Button {...restProps} {size} type="button" {variant} class={cn("size-7", className)}>
		{@render children?.()}
		{#if srOnlyLabel}
			<span class="sr-only">{srOnlyLabel}</span>
		{/if}
	</Button>
{/if}