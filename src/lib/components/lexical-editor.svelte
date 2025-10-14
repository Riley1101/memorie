<script>
  import { onMount } from "svelte";
  import { Composer, ContentEditable, RichTextPlugin } from "svelte-lexical";
  import { theme } from "svelte-lexical/dist/themes/system-light-dark";
  import { Button } from "$lib/components/ui/button/index.js";
  import { fileManager } from "$lib/runes/fs.svelte";

  /**
   * @type {string | null}
   * Serialized Lexical editor state (JSON string).
   */
  export let nodes = null;

  /**
   * @type {string | null}
   * Name of the file being edited.
   */
  export let name = "";

  const initialConfig = {
    theme,
    namespace: "memorie-lexical-editor",
    /**
     * @param {Error} error
     */
    onError: (error) => {
      console.error("Lexical editor error:", error);
      throw error;
    },
  };

  /** @type {import("svelte-lexical").Composer | null} */
  let composer = null;

  /**
   * Update the editor state if nodes are provided.
   */
  function setEditorState() {
    if (!nodes || !composer) return;
    try {
      const editor = composer.getEditor();
      const parsed = JSON.parse(nodes);
      editor.setEditorState(editor.parseEditorState(parsed));
    } catch (err) {
      console.warn("Failed to load editor state:", err);
    }
  }

  function saveContent() {
    const editor = composer?.getEditor();
    const editorState = editor?.getEditorState();
    const serialized = JSON.stringify(editorState);
    if(name){
      fileManager.saveCurrentFile(name || '', serialized);
      setEditorState()
    }
  }

  onMount(() => setEditorState());

  $: if (nodes) setEditorState();

</script>

<Button onclick={saveContent} class="mb-2">Save</Button>

<Composer {initialConfig} bind:this={composer}>
  <div class="text-foreground p-2 border bg-card relative">
    <ContentEditable />
    <RichTextPlugin />
  </div>
</Composer>
