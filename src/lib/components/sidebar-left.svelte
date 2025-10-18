<script>
    import HouseIcon from "@lucide/svelte/icons/house";
    import SearchIcon from "@lucide/svelte/icons/search";
    import SparklesIcon from "@lucide/svelte/icons/sparkles";

    import NavFavorites from "./nav-favorites.svelte";
    import NavMain from "./nav-main.svelte";
    import TeamSwitcher from "./team-switcher.svelte";
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";


    const data = {
        navMain: [
            {
                title: "Search",
                url: "#",
                icon: SearchIcon,
            },
            {
                title: "Ask AI",
                url: "/chat",
                icon: SparklesIcon,
            },
            {
                title: "Home",
                url: "/",
                icon: HouseIcon
            },
        ],
    };

    let { favourites = [], ref = $bindable(null), ...restProps } = $props();
</script>

<Sidebar.Root class="border-r-0" {...restProps}>
    <Sidebar.Header>
        <TeamSwitcher />
        <NavMain items={data.navMain} />
    </Sidebar.Header>
    <Sidebar.Content>
        <NavFavorites {favourites} />
    </Sidebar.Content>
    <Sidebar.Rail />
</Sidebar.Root>
