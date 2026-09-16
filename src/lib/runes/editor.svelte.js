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
   * @type {{ level: number, text: string, pos: number }[]} - Headings of the open document, in order.
   */
  headings = $state([]);

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
   * Scene metadata of the open writing (from its front matter).
   * @type {import('$lib/front-matter.js').SceneMeta}
   */
  meta = $state({});

  /**
   * Front matter lines the app doesn't understand, written back unchanged.
   * @type {string[]}
   */
  metaExtra = [];

  /**
   * Metadata and front matter block as loaded, so an unchanged block is saved
   * back exactly as it was.
   * @type {{ meta: import('$lib/front-matter.js').SceneMeta, raw: string }}
   */
  metaOriginal = { meta: {}, raw: '' };

  /**
   * Replaces the metadata when a writing is (re)loaded. Doesn't save.
   * @param {import('$lib/front-matter.js').ParsedWriting} parsed
   */
  loadMeta(parsed) {
    this.meta = parsed.meta;
    this.metaExtra = parsed.extra;
    this.metaOriginal = { meta: parsed.meta, raw: parsed.raw };
  }

  /**
   * Changes metadata (synopsis, status…) and saves the writing.
   * @param {Partial<import('$lib/front-matter.js').SceneMeta>} patch
   */
  async updateMeta(patch) {
    this.meta = { ...this.meta, ...patch };
    if (this.flushSave) await this.flushSave();
  }

  /**
   * A search match to select once the target writing's editor is ready.
   * `occurrence` is the match's index within that writing.
   * @type {{ query: string, options: { caseSensitive: boolean, wholeWord: boolean, regex: boolean }, occurrence: number } | null}
   */
  pendingReveal = null;

  /**
   * Registered by the mounted editor: selects `pendingReveal` in the open
   * writing. Null when no editor is open.
   * @type {(() => void) | null}
   */
  revealPending = null;

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
   * Replaces the headings list, skipping no-op updates so the outline doesn't re-render on every keystroke.
   * @param next {{ level: number, text: string, pos: number }[]}
   */
  setHeadings(next) {
    const same =
      next.length === this.headings.length &&
      next.every((h, i) => {
        const cur = this.headings[i];
        return cur.level === h.level && cur.text === h.text && cur.pos === h.pos;
      });
    if (!same) this.headings = next;
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
