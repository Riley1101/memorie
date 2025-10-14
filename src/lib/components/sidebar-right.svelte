<script>
  import NavUser from "./nav-user.svelte";
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import {Button} from "@/components/ui/button/index.js";
  import { fileManager } from "$lib/runes/fs.svelte";
  import {invoke} from "@tauri-apps/api/core";
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

  let { ref = $bindable(null), ...restProps } = $props();

  function handleCreateEmbeddings (){
    const currentContent = fileManager.currentContent;
    const currentFile = fileManager.currentFile;

    let content = `
Ode to React

In the DOM’s vast, tangled sea,
A library came to set us free.
Components small, yet mighty in might,
Bringing UIs to glorious light.

JSX, a syntax of wonder and grace,
Merging logic with a visual space.
Props pass data, state holds the key,
To dynamic pages, swift and free.

Hooks appear, a modern delight,
useState, useEffect—coding takes flight.
Virtual DOM, so clever and fast,
Ensuring updates never last.

From simple buttons to complex app,
React builds worlds with each small map.
A declarative promise, elegant and true,
In the web we craft, it sees us through.
    `

    invoke("create_embeddings", { name: "welcome.lexical" , content: content  }).then((res) => {
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
    <div class="p-2">
      <Button onclick={handleCreateEmbeddings}>
        Create Embeddings
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

