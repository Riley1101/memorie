<script>
    import { Color } from '@tiptap/extension-text-style'
    import { ListItem } from '@tiptap/extension-list'
    import { TextStyle } from '@tiptap/extension-text-style'
    import StarterKit from "@tiptap/starter-kit";
    import { Editor } from "@tiptap/core";
    import { onMount } from "svelte";
    import { Markdown } from '@tiptap/markdown'
    import MdToolbar from './md-toolbar.svelte';
    import BubbleMenu from '@tiptap/extension-bubble-menu';
    import {Button} from "@/components/ui/button/index.js";

    /** @type {HTMLDivElement | undefined} */
    let element = $state()

    /** @type {Editor} */
    let editor;

    /** @type {{editor: Editor | null}} */
    let editorState = $state({editor: null})

    $inspect(editorState);

    /**
     * @type {{content: string | [], onSave: (content: string) => void}}
     */
    let { content = [], onSave } = $props();

    onMount(() => {
        editor = new Editor({
            element: element,
            extensions: [
                Color.configure({ types: [TextStyle.name, ListItem.name] }),
                TextStyle.configure({ types: [ListItem.name] }),
                StarterKit,
                Markdown,
            ],
            content: content || [],
            contentType:"markdown",
            onTransaction: ({editor}) => {
                editorState = { editor }
            },
        });
    });
</script>

<Button class="mb-2" onclick={()=>{
    const content =editor.getMarkdown();
    onSave(content);
}}> Save </Button>

<MdToolbar editor={editor} />

<div bind:this={element} class="border pl-4 pt-4 rounded-lg  max-w-none h-[40vh] border-t-none w-full"></div>


<style>
    :global(.ProseMirror:focus) {
        outline: none;
    }

    :global(.ProseMirror) {
        min-height: 300px;
    }
</style>