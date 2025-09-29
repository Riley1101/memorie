<script>
    import AudioWaveformIcon from "@lucide/svelte/icons/audio-waveform";
    import BlocksIcon from "@lucide/svelte/icons/blocks";
    import CalendarIcon from "@lucide/svelte/icons/calendar";
    import CommandIcon from "@lucide/svelte/icons/command";
    import HouseIcon from "@lucide/svelte/icons/house";
    import InboxIcon from "@lucide/svelte/icons/inbox";
    import MessageCircleQuestionIcon from "@lucide/svelte/icons/message-circle-question";
    import SearchIcon from "@lucide/svelte/icons/search";
    import Settings2Icon from "@lucide/svelte/icons/settings-2";
    import SparklesIcon from "@lucide/svelte/icons/sparkles";
    import Trash2Icon from "@lucide/svelte/icons/trash-2";

    import NavFavorites from "./nav-favorites.svelte";
    import NavMain from "./nav-main.svelte";
    import NavSecondary from "./nav-secondary.svelte";
    import NavWorkspaces from "./nav-workspaces.svelte";
    import TeamSwitcher from "./team-switcher.svelte";
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    // This is sample data.
    const data = {
        teams: [
            {
                name: "Acme Inc",
                logo: CommandIcon,
                plan: "Enterprise",
            },
            {
                name: "Acme Corp.",
                logo: AudioWaveformIcon,
                plan: "Startup",
            },
            {
                name: "Evil Corp.",
                logo: CommandIcon,
                plan: "Free",
            },
        ],
        navMain: [
            {
                title: "Search",
                url: "#",
                icon: SearchIcon,
            },
            {
                title: "Ask AI",
                url: "#",
                icon: SparklesIcon,
            },
            {
                title: "Home",
                url: "/",
                icon: HouseIcon,
                isActive: true,
            },
            {
                title: "Inbox",
                url: "#",
                icon: InboxIcon,
                badge: "10",
            },
        ],
        navSecondary: [
            {
                title: "Calendar",
                url: "#",
                icon: CalendarIcon,
            },
            {
                title: "Settings",
                url: "#",
                icon: Settings2Icon,
            },
            {
                title: "Templates",
                url: "#",
                icon: BlocksIcon,
            },
            {
                title: "Trash",
                url: "#",
                icon: Trash2Icon,
            },
            {
                title: "Help",
                url: "#",
                icon: MessageCircleQuestionIcon,
            },
        ],
        favorites: [
            {
                name: "Project Management & Task Tracking",
                url: "#",
                emoji: "📊",
            },
            {
                name: "Family Recipe Collection & Meal Planning",
                url: "#",
                emoji: "🍳",
            },
        ],
    };

    let { favourites = [], ref = $bindable(null), ...restProps } = $props();
</script>

<Sidebar.Root class="border-r-0" {...restProps}>
    <Sidebar.Header>
        <TeamSwitcher teams={data.teams} />
        <NavMain items={data.navMain} />
    </Sidebar.Header>
    <Sidebar.Content>
        <NavFavorites {favourites} />
        <!-- <NavWorkspaces workspaces={data.workspaces} /> -->
        <!-- <NavSecondary items={data.navSecondary} class="mt-auto" /> -->
    </Sidebar.Content>
    <Sidebar.Rail />
</Sidebar.Root>
