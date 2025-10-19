<script>
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import { fileManager } from "$lib/runes/fs.svelte";
  import {invoke} from "@tauri-apps/api/core";

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
  let isOpen = $state(true);

  function handleSearchEmbeddings(){
    invoke("search_embeddings", { query }).then((res) => {
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
  side="right"
  collapsible="icon"
  class="sticky top-0 hidden h-svh border-r lg:flex"
  {...restProps}
>
  <Sidebar.Header>
  </Sidebar.Header>
  <Sidebar.Content></Sidebar.Content>
  <Sidebar.Rail />
</Sidebar.Root>

