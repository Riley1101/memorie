<script>
    import { core } from "@tauri-apps/api";
    import { page } from "$app/stores";
    import { fileManager } from "$lib/runes/fs.svelte";
    const { lexical } = $page.params;
    const { readFile } = fileManager;
    readFile({
        path: "./welcome.lexical",
        name: "lexical",
    });
    $inspect(fileManager);
</script>

<main class="container">
    <code class="border p-4 rounded-md"> Detail Lexical Note </code>
</main>
