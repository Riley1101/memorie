<script>
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Tooltip from "$lib/components/ui/tooltip/index.js";
	import { cn } from "$lib/utils";
	/** @type {{ tooltip?: any, label?: any, variant?: "link" | "default" | "outline" | "secondary" | "ghost" | "destructive", size?: "icon" | "default" | "xs" | "sm" | "lg" | "icon-xs" | "icon-sm" | "icon-lg", class?: any, children?: any, [key: string]: any }} */
	let {
		tooltip,
		label,
		variant = "ghost",
		size = "icon",
		class: className = undefined,
		children = undefined,
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