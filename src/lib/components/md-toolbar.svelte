<script>
    import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";
    import BoldIcon from "@lucide/svelte/icons/bold";
    import SaveIcon from "@lucide/svelte/icons/save";
    import ItalicIcon from "@lucide/svelte/icons/italic";
    import UnderlineIcon from "@lucide/svelte/icons/underline";
    /** @type {import('@tiptap/core').Editor | undefined } */
    export let editor;

    /**
     * @type {() => void} onSave
     */
    export let onSave;
</script>

{#if editor}
    <div class="mb-4">
    <ToggleGroup.Root type="multiple" variant="outline">
        <ToggleGroup.Item value="bold" aria-label="Toggle bold" onclick={() => editor.chain().focus().toggleBold().run()} >
            <BoldIcon class="h-4 w-4" />
        </ToggleGroup.Item>
        <ToggleGroup.Item value="italic" aria-label="Toggle italic" onclick={() => editor.chain().focus().toggleItalic().run()}>
            <ItalicIcon class="h-4 w-4" />
        </ToggleGroup.Item>
        <ToggleGroup.Item value="strikethrough" aria-label="Toggle strikethrough" onclick={() => editor.chain().focus().toggleStrike().run()}>
            <UnderlineIcon class="h-4 w-4" />
        </ToggleGroup.Item>

        <ToggleGroup.Item value="strikethrough" aria-label="Toggle strikethrough" onclick={onSave}>
            <SaveIcon class="h-4 w-4" />
        </ToggleGroup.Item>
    </ToggleGroup.Root>
    </div>

<!--    <div class="control-group">-->
<!--        <div class="button-group">-->
<!--            <button-->
<!--                    on:click={() => console.log && editor.chain().focus().toggleBold().run()}-->
<!--                    disabled={!editor.can().chain().focus().toggleBold().run()}-->
<!--                    class={editor.isActive("bold") ? "is-active" : ""}-->
<!--            >-->
<!--                Bold-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleItalic().run()}-->
<!--                    disabled={!editor.can().chain().focus().toggleItalic().run()}-->
<!--                    class={editor.isActive("italic") ? "is-active" : ""}-->
<!--            >-->
<!--                Italic-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleStrike().run()}-->
<!--                    disabled={!editor.can().chain().focus().toggleStrike().run()}-->
<!--                    class={editor.isActive("strike") ? "is-active" : ""}-->
<!--            >-->
<!--                Strike-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleCode().run()}-->
<!--                    disabled={!editor.can().chain().focus().toggleCode().run()}-->
<!--                    class={editor.isActive("code") ? "is-active" : ""}-->
<!--            >-->
<!--                Code-->
<!--            </button>-->
<!--            <button on:click={() => editor.chain().focus().unsetAllMarks().run()}>Clear marks</button>-->
<!--            <button on:click={() => editor.chain().focus().clearNodes().run()}>Clear nodes</button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().setParagraph().run()}-->
<!--                    class={editor.isActive("paragraph") ? "is-active" : ""}-->
<!--            >-->
<!--                Paragraph-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}-->
<!--                    class={editor.isActive("heading", { level: 1 }) ? "is-active" : ""}-->
<!--            >-->
<!--                H1-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}-->
<!--                    class={editor.isActive("heading", { level: 2 }) ? "is-active" : ""}-->
<!--            >-->
<!--                H2-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}-->
<!--                    class={editor.isActive("heading", { level: 3 }) ? "is-active" : ""}-->
<!--            >-->
<!--                H3-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleHeading({ level: 4 }).run()}-->
<!--                    class={editor.isActive("heading", { level: 4 }) ? "is-active" : ""}-->
<!--            >-->
<!--                H4-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleHeading({ level: 5 }).run()}-->
<!--                    class={editor.isActive("heading", { level: 5 }) ? "is-active" : ""}-->
<!--            >-->
<!--                H5-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleHeading({ level: 6 }).run()}-->
<!--                    class={editor.isActive("heading", { level: 6 }) ? "is-active" : ""}-->
<!--            >-->
<!--                H6-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleBulletList().run()}-->
<!--                    class={editor.isActive("bulletList") ? "is-active" : ""}-->
<!--            >-->
<!--                Bullet list-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleOrderedList().run()}-->
<!--                    class={editor.isActive("orderedList") ? "is-active" : ""}-->
<!--            >-->
<!--                Ordered list-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleCodeBlock().run()}-->
<!--                    class={editor.isActive("codeBlock") ? "is-active" : ""}-->
<!--            >-->
<!--                Code block-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().toggleBlockquote().run()}-->
<!--                    class={editor.isActive("blockquote") ? "is-active" : ""}-->
<!--            >-->
<!--                Blockquote-->
<!--            </button>-->
<!--            <button on:click={() => editor.chain().focus().setHorizontalRule().run()}>-->
<!--                Horizontal rule-->
<!--            </button>-->
<!--            <button on:click={() => editor.chain().focus().setHardBreak().run()}>Hard break</button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().undo().run()}-->
<!--                    disabled={!editor.can().chain().focus().undo().run()}-->
<!--            >-->
<!--                Undo-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().redo().run()}-->
<!--                    disabled={!editor.can().chain().focus().redo().run()}-->
<!--            >-->
<!--                Redo-->
<!--            </button>-->
<!--            <button-->
<!--                    on:click={() => editor.chain().focus().setColor('#958DF1').run()}-->
<!--                    class={editor.isActive('textStyle', { color: '#958DF1' }) ? 'is-active' : ''}-->
<!--            >-->
<!--                Purple-->
<!--            </button>-->
<!--        </div>-->
<!--    </div>-->
{/if}