<script>
	import { Dialog as DialogPrimitive } from "bits-ui";

	/** @type {{ ref?: any, type?: "button" | "reset" | "submit", [key: string]: any }} */
	let {
		ref = $bindable(null),
		type = "button",
		...restProps
	} = $props();
</script>

<DialogPrimitive.Trigger bind:ref data-slot="dialog-trigger" {type} {...restProps} />