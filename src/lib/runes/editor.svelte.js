/**
 * @file Editor state management using Svelte's reactive stores.
 */

class EditorState {
  /**
   * @public
   * @type {import("@tiptap/core").Editor} - The Tiptap editor instance.
   */
  editor = $state(null);

  /**
   * Sets the editor instance.
   * @param editorInstance {Editor} - The Tiptap editor instance.
   */
  setEditor(editorInstance) {
    this.editor = editorInstance;
  }
}

export let editorState = new EditorState();
