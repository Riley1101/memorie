/**
 * @file Editor state management using Svelte's reactive stores.
 */

class EditorState {

  // --- STATE ---
  name = $state('Untitled Document');
  content = $state('');

  /**
   * @public
   * @type {import("@milkdown/kit/core").Editor | null} - The Tiptap editor instance.
   */
  editor = $state(null);

  /**
   * @public
   * @type {boolean} - Indicates if the editor is in edit mode.
   */
  editMode = $state(false);

  /**
   * @public
   * @type {{lastSaved: null, status: string}}
   */
  saveStatus = $state({
    lastSaved: null,
    status: 'idle',
  });

  /**
   * Sets the editor instance.
   * @param editorInstance {import("@milkdown/kit/core").Editor} - The Tiptap editor instance.
   */
  setEditor(editorInstance) {
    this.editor = editorInstance;
  }

  /**
   * Sets the save status.
   * @param status
   */
  setSaveStatus(status) {
    this.saveStatus = { ...this.saveStatus, ...status };
  }

  /**
   * Sets the edit mode.
   * @param isEditMode boolean - True to enable edit mode, false to disable.
   */
  setEditMode(isEditMode) {
    this.editMode = isEditMode;
  }

  /**
   * Sets the document name.
   * @param newName string - The new document name.
   */
  setName(newName) {
    this.name = newName;
  }

  /**
   * Sets the document content.
   * @param newContent string - The new document content.
   */
  setContent(newContent) {
    this.content = newContent;
  }
}

export let editorState = new EditorState();
