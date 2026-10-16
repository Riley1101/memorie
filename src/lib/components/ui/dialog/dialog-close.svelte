<script>
	import { Dialog as DialogPrimitive } from "bits-ui";

	/** @type {{ ref?: any, type?: "button" | "reset" | "submit", [key: string]: any }} */
	let {
		ref = $bindable(null),
		type = "button",
		...restProps
	} = $props();
</script>

<DialogPrimitive.Close bind:ref data-slot="dialog-close" {type} {...restProps} />