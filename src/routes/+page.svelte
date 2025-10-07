<script>
    import { Button } from "@/components/ui/button";
    import { Input } from "@/components/ui/input";
    import { fileManager } from "$lib/runes/fs.svelte";

    let fileName = "";

    function createFile() {
      fileManager.createNewFile(fileName);
    }
</script>

<main class="container">
    <div class="flex flex-col gap-4">
        <Input placeholder="File Name" bind:value={fileName}/>
        <Button class="max-w-max" onclick={createFile}>
            Create New File
        </Button>
    </div>
</main>
