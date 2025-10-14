<script>
  import NavUser from "./nav-user.svelte";
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import {Button} from "@/components/ui/button/index.js";
  import { fileManager } from "$lib/runes/fs.svelte";
  import {invoke} from "@tauri-apps/api/core";
  import {Input} from "@/components/ui/input/index.js";
  // This is sample data.
  const data = {
    user: {
      name: "shadcn",
      email: "m@example.com",
    },
    calendars: [
      {
        name: "My Calendars",
        items: ["Personal", "Work", "Family"],
      },
      {
        name: "Favorites",
        items: ["Holidays", "Birthdays"],
      },
      {
        name: "Other",
        items: ["Travel", "Reminders", "Deadlines"],
      },
    ],
  };

  let query = $state("");

  let { ref = $bindable(null), ...restProps } = $props();

  function handleSearchEmbeddings(){
    invoke("search_embeddings", { query }).then((res) => {
      console.log("Search results:", res);
    }).catch((err) => {
      console.error("Error searching embeddings:", err);
    });
  }

  function handleCreateEmbeddings (){
    const currentContent = fileManager.currentContent;
    const currentFile = fileManager.currentFile;

    invoke("create_embeddings", { name: currentFile , content: currentContent  }).then((res) => {
      console.log("Embeddings created:", res);
    }).catch((err) => {
      console.error("Error creating embeddings:", err);
    });
  }

</script>

<Sidebar.Root
  bind:ref
  collapsible="none"
  class="sticky top-0 hidden h-svh border-l lg:flex"
  {...restProps}
>
  <Sidebar.Header class="border-sidebar-border h-16 border-b">
    <NavUser user={data.user} />
  </Sidebar.Header>
  <Sidebar.Content>
    <div class="p-2 flex flex-col gap-2 max-w-max">
      <Input bind:value={query}/>

      <Button onclick={handleCreateEmbeddings}>
        Create Embeddings
      </Button>
      <Button onclick={handleSearchEmbeddings}>
        Search Embeddings
      </Button>
    </div>
  </Sidebar.Content>
  <Sidebar.Footer>
    <Sidebar.Menu>
      <Sidebar.MenuItem>
        <Sidebar.MenuButton>
          <!-- <PlusIcon /> -->
          <!-- <span>New Calendar</span> -->
        </Sidebar.MenuButton>
      </Sidebar.MenuItem>
    </Sidebar.Menu>
  </Sidebar.Footer>
</Sidebar.Root>

