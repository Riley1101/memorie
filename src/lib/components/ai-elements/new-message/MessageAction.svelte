<script>
	import { cn } from "$lib/utils";
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Tooltip from "$lib/components/ui/tooltip/index.js";
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
</script>

{#if tooltip}
	<Tooltip.Provider>
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						{size}
						type="button"
						{variant}
						class={cn("size-7", className)}
						{...restProps}
					>
						{@render children?.()}
						<span class="sr-only">{label || tooltip}</span>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content>
				<p>{tooltip}</p>
			</Tooltip.Content>
		</Tooltip.Root>
	</Tooltip.Provider>
{:else}
	<Button {size} type="button" {variant} class={cn("size-7", className)} {...restProps}>
		{@render children?.()}
		{#if label}
			<span class="sr-only">{label}</span>
		{/if}
	</Button>
{/if}