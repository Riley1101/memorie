<script>
    import SidebarLeft from "$lib/components/sidebar-left.svelte";
    import SidebarRight from "$lib/components/sidebar-right.svelte";
    import * as Breadcrumb from "$lib/components/ui/breadcrumb/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import { fileManager } from "$lib/runes/fs.svelte";
    import "../app.css";

    /**
     * Initializes the file manager and retrieves the list of files.
     */
    fileManager.getFiles();

    let { children } = $props();
</script>

<Sidebar.Provider>
    <SidebarLeft favourites={fileManager.files} />
    <Sidebar.Inset>
        <header
            class="bg-background sticky top-0 flex h-14 shrink-0 items-center gap-2"
        >
            <div class="flex flex-1 items-center gap-2 px-3">
                <Sidebar.Trigger />
                <Separator
                    orientation="vertical"
                    class="mr-2 data-[orientation=vertical]:h-4"
                />
                <Breadcrumb.Root>
                    <Breadcrumb.List>
                        <Breadcrumb.Item>
                            <Breadcrumb.Page class="line-clamp-1"
                                >Welcome</Breadcrumb.Page
                            >
                        </Breadcrumb.Item>
                    </Breadcrumb.List>
                </Breadcrumb.Root>
            </div>
        </header>
        <div class="flex flex-1 flex-col gap-4 p-4">
            {@render children()}
        </div>
    </Sidebar.Inset>
    <SidebarRight />
</Sidebar.Provider>
