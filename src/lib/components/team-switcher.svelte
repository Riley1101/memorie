<script>
	import * as Sidebar from "$lib/components/ui/sidebar/index.js";
	import CommandIcon from "@lucide/svelte/icons/command";

	let team = {
		name: "Memorie",
				logo: CommandIcon,
			plan: "Just You",
	};

</script>

<Sidebar.Menu>
	<Sidebar.MenuItem>
		<Sidebar.MenuButton class="w-fit px-1.5">
			<div
					class="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-5 items-center justify-center rounded-md"
			>
				<team.logo class="size-3" />
			</div>
			<span class="truncate font-medium text-lg">{team.name}</span>
		</Sidebar.MenuButton>
	</Sidebar.MenuItem>
</Sidebar.Menu>