/**
 * @file Editor state management using Svelte's reactive stores.
 */

class EditorState {
  /**
   * @public
   * @type {import("@tiptap/core").Editor | null} - The Tiptap editor instance.
   */
  editor = $state(null);

  /**
   * Sets the editor instance.
   * @param editorInstance {import("@tiptap/core").Editor} - The Tiptap editor instance.
   */
  setEditor(editorInstance) {
    this.editor = editorInstance;
  }
}

export let editorState = new EditorState();
