<script>
	let { class: className = "", size = 16, ...restProps } = $props();
</script>

<svg
	{...restProps}
	class={className}
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
>
	<path d="M21 12a9 9 0 1 1-6.219-8.56" />
</svg>