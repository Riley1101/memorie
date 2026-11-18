import { editorState } from '$lib/runes/editor.svelte.js';

/**
 * @file Margin notes on the open writing: notes a writer leaves themselves on
 * a stretch of text ("is the bell too on-the-nose?").
 *
 * They are kept in the writing's front matter, so they travel with the file
 * and never reach an export. A note remembers the text it was left on
 * (`quote`) and a little of what came before it (`prefix`), which is enough to
 * find its place again when the writing is reopened. While the writing is open
 * the editor tracks the place itself (plugins/margin-notes.js) and keeps
 * `quote` up to date as the text under the note is edited.
 */

/**
 * @typedef {Object} Note
 * @property {string} id
 * @property {string} quote - The text the note is on, as it was last saved.
 * @property {string} prefix - A little of the text just before it, to tell repeats apart.
 * @property {string} text - What the writer wrote.
 * @property {string} at - When it was left (ISO 8601).
 */

/** @param {unknown} n @returns {n is Note} */
function isNote(n) {
  const note = /** @type {Record<string, unknown>} */ (n);
  return (
    !!note &&
    typeof note.id === 'string' &&
    typeof note.quote === 'string' &&
    note.quote !== '' &&
    typeof note.text === 'string'
  );
}

/**
 * Reads notes from front matter. Anything malformed is skipped rather than
 * failing the whole writing.
 * @param {string | undefined} raw
 * @returns {Note[]}
 */
export function parseNotes(raw) {
  if (!raw) return [];
  try {
    const value = JSON.parse(raw);
    if (!Array.isArray(value)) return [];
    return value.filter(isNote).map((n) => ({
      id: n.id,
      quote: n.quote,
      prefix: typeof n.prefix === 'string' ? n.prefix : '',
      text: n.text,
      at: typeof n.at === 'string' ? n.at : '',
    }));
  } catch {
    return [];
  }
}

/** @param {Note[]} notes @returns {string} '' when there are none, so the key is left out. */
export function serializeNotes(notes) {
  if (notes.length === 0) return '';
  return JSON.stringify(notes.map(({ id, quote, prefix, text, at }) => ({ id, quote, prefix, text, at })));
}

class NotesState {
  /** @type {Note[]} */
  list = $state([]);

  /**
   * Where each placed note is in the open document, by id. A note missing
   * here could not be placed: the text it was on is gone.
   * @type {Record<string, { from: number, to: number }>}
   */
  anchors = $state({});

  /** The note under the caret, if any. */
  activeId = $state(/** @type {string | null} */ (null));

  /** The note whose text is being written or edited. */
  editingId = $state(/** @type {string | null} */ (null));

  /**
   * Whether there is room to show notes in the margin beside the page. When
   * there isn't, the scene panel shows them instead.
   */
  marginVisible = $state(false);

  /** Bumped whenever the document moves under the notes, so the margin can re-measure. */
  layoutVersion = $state(0);

  /** Notes whose text could not be found, newest first. */
  detached = $derived(this.list.filter((n) => !(n.id in this.anchors)));

  /**
   * Replaces the notes when a writing is (re)loaded. Doesn't save.
   * @param {string | undefined} raw - The `notes` front matter value.
   */
  load(raw) {
    this.list = parseNotes(raw);
    this.anchors = {};
    this.activeId = null;
    this.editingId = null;
  }

  /** Puts the notes back into the writing's metadata for the next save, without saving now. */
  stage() {
    editorState.meta = { ...editorState.meta, notes: serializeNotes(this.list) };
  }

  /** Puts the notes back into the metadata and saves the writing. */
  async persist() {
    await editorState.updateMeta({ notes: serializeNotes(this.list) });
  }

  /**
   * Called by the editor as the document changes: where the notes are now,
   * and the current text under each one.
   * @param {Record<string, { from: number, to: number, quote: string, prefix: string }>} placed
   * @param {string | null} activeId
   */
  sync(placed, activeId) {
    /** @type {Record<string, { from: number, to: number }>} */
    const anchors = {};
    let changed = false;
    for (const note of this.list) {
      const at = placed[note.id];
      if (!at) continue;
      anchors[note.id] = { from: at.from, to: at.to };
      if (at.quote && (at.quote !== note.quote || at.prefix !== note.prefix)) {
        note.quote = at.quote;
        note.prefix = at.prefix;
        changed = true;
      }
    }
    this.anchors = anchors;
    this.activeId = activeId;
    this.layoutVersion++;
    // The document change that moved the text is already being autosaved.
    if (changed) this.stage();
  }

  /**
   * Starts a note on a stretch of text; the writer then types it.
   * @param {{ quote: string, prefix: string }} where
   * @returns {Note}
   */
  create(where) {
    /** @type {Note} */
    const note = {
      id: crypto.randomUUID().slice(0, 8),
      quote: where.quote,
      prefix: where.prefix,
      text: '',
      at: new Date().toISOString(),
    };
    this.list = [...this.list, note];
    this.editingId = note.id;
    return note;
  }

  /**
   * Sets a note's text. An empty note is thrown away instead.
   * @param {string} id
   * @param {string} text
   * @returns {Promise<boolean>} false when the note was removed for being empty.
   */
  async write(id, text) {
    const note = this.list.find((n) => n.id === id);
    if (this.editingId === id) this.editingId = null;
    if (!note) return false;
    const trimmed = text.trim();
    if (!trimmed) {
      await this.remove(id);
      return false;
    }
    if (note.text === trimmed) return true;
    note.text = trimmed;
    await this.persist();
    return true;
  }

  /** @param {string} id */
  async remove(id) {
    const wasSaved = this.list.some((n) => n.id === id && n.text);
    this.list = this.list.filter((n) => n.id !== id);
    if (this.editingId === id) this.editingId = null;
    if (this.activeId === id) this.activeId = null;
    this.anchors = Object.fromEntries(Object.entries(this.anchors).filter(([key]) => key !== id));
    // A note that was never written was never saved either.
    if (wasSaved) await this.persist();
  }
}

export const notesState = new NotesState();
