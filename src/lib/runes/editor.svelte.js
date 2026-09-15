/**
 * @file Editor state management using Svelte's reactive stores.
 */

/**
 * @typedef {Object} RecordPointer
 * @property {string} tb - The table name (e.g., 'chunk', 'documents').
 * @property {string} id - The record identifier wrapper.
 */

/**
 * @typedef {Object} ChunkItem
 * @property {RecordPointer} id - The unique identifier for this chunk.
 * @property {RecordPointer} parent - The reference to the parent document.
 * @property {string} content - The text content of the paragraph.
 * @property {number} sequence - The order of the paragraph (0-indexed).
 * @property {string} content_hash - A hash string for the content.
 * @property {?Object} grammar_check - Grammar check results (nullable).
 * @property {boolean} is_dirty - Indicates if the content has been modified.
 */

class EditorState {
  // --- STATE ---
  name = $state('Untitled Writing');
  content = $state('');
  /**
   * @public
   * @type {ChunkItem[]} - Array of chunk items representing paragraphs.
   */
  grammarChecks = $state([]);

  /**
   * @public
   * @type {import("@milkdown/kit/core").Editor | null} - The Milkdown editor instance.
   */
  editor = $state(null);

  /**
   * @public
   * @type {{lastSaved: null, status: string}}
   */
  saveStatus = $state({
    lastSaved: null,
    status: 'idle',
  });

  /**
   * Set right before a same-document URL change (draft materialised, auto
   * rename) so the editor page keeps the live editor instead of remounting.
   * @type {boolean}
   */
  silentNavigation = $state(false);

  /**
   * Registered by the mounted editor: saves immediately, bypassing the
   * autosave debounce. Null when no editor is open.
   * @type {(() => Promise<void>) | null}
   */
  flushSave = $state(null);

  /**
   * Sets the editor instance.
   * @param editorInstance {import("@milkdown/kit/core").Editor} - The Milkdown editor instance.
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

  /**
   * Sets the grammar checks.
   * @param checks {ChunkItem[]} - Array of chunk items with grammar checks.
   */
  setGrammarChecks(checks) {
    this.grammarChecks = checks;
  }
}

export let editorState = new EditorState();
