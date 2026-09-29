/**
 * @file Client for the Rust document engine (`src-tauri/src/editor`).
 *
 * The engine owns the document; this module is the only way the frontend talks
 * to it. Nothing in the UI uses it yet — Milkdown is still the editor the
 * writer types into — so this is the seam the migration moves onto, not a
 * second live editor. See `docs/editor-core.md`.
 *
 * Offsets in every position here are **UTF-16 code units**, the same thing a
 * JavaScript string index means, so `text.slice(offset)` lines up. The Rust
 * side converts to its own character offsets at the boundary.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * @typedef {Object} EditorPosition
 * @property {number} block - Block id; stable across edits to other blocks.
 * @property {number} offset - UTF-16 offset inside that block's inline content.
 */

/**
 * @typedef {Object} EditorRun
 * @property {'text' | 'image' | 'html' | 'break'} type
 * @property {string} [text]
 * @property {number} length - In UTF-16 units. An image, break or html run is 1.
 * @property {string[]} [marks] - 'bold' | 'italic' | 'code' | 'strike'.
 * @property {string} [href]
 * @property {string} [title]
 * @property {string} [source] - Image source.
 * @property {string} [alt]
 * @property {string} [html]
 */

/**
 * @typedef {Object} EditorBlock
 * @property {number} id
 * @property {string} kind - paragraph | heading | codeBlock | quote | list | image | table | thematicBreak | html
 * @property {EditorRun[]} [runs]
 * @property {number} [level]
 * @property {string} [language]
 * @property {string} [text] - A code block's code, or raw HTML.
 * @property {boolean} [ordered]
 * @property {number} [start]
 * @property {boolean} [tight]
 * @property {EditorBlock[]} [children] - Blocks inside a quote.
 * @property {{ checked?: boolean, blocks: EditorBlock[] }[]} [items]
 * @property {{ cells: EditorBlock[][] }[]} [rows]
 * @property {string[]} [alignments]
 */

/**
 * @typedef {Object} EditorStateView
 * @property {string} name
 * @property {number} revision - Moves on every edit; stale state is detectable.
 * @property {{ anchor: EditorPosition, head: EditorPosition }} selection
 * @property {EditorBlock[]} blocks
 * @property {{ id: number, level: number, text: string }[]} outline
 * @property {string[]} activeMarks
 * @property {number} wordCount
 * @property {boolean} canUndo
 * @property {boolean} canRedo
 * @property {Record<string, string>} [frontmatter]
 */

/**
 * Commands the engine accepts. One shape per editing action; the frontend never
 * names an operation or a transaction.
 * @typedef {
 *   | { command: 'setSelection', anchor: EditorPosition, head: EditorPosition }
 *   | { command: 'insertText', text: string }
 *   | { command: 'reconcileBlock', block: number, text: string }
 *   | { command: 'delete', forward?: boolean }
 *   | { command: 'splitBlock' }
 *   | { command: 'mergeBlocks' }
 *   | { command: 'toggleMark', mark: 'bold' | 'italic' | 'code' | 'strike' }
 *   | { command: 'setHeadingLevel', level: number }
 *   | { command: 'setParagraph' }
 *   | { command: 'setCodeBlock', language?: string | null }
 *   | { command: 'wrapInQuote' }
 *   | { command: 'toggleList', ordered?: boolean }
 *   | { command: 'toggleTask' }
 *   | { command: 'insertTable', rows: number, columns: number }
 *   | { command: 'insertTableRow', before?: boolean }
 *   | { command: 'insertTableColumn', before?: boolean }
 *   | { command: 'setLink', url?: string | null }
 *   | { command: 'insertImage', source: string, alt?: string }
 *   | { command: 'insertParagraph' }
 *   | { command: 'insertThematicBreak' }
 *   | { command: 'undo' }
 *   | { command: 'redo' }
 * } EditorCommand
 */

/**
 * Opens a writing in the engine, or returns the session already open for it.
 * Unsaved edits are kept unless `reload` is set.
 * @param {string} name - The writing's path, as everywhere else in the app.
 * @param {{ reload?: boolean }} [options]
 * @returns {Promise<EditorStateView>}
 */
export function openDocument(name, options = {}) {
  return invoke('editor_open', { name, reload: options.reload ?? false });
}

/**
 * Opens a document from text rather than from disk, replacing any session under
 * that name. For content with no file yet — an import preview, a benchmark, a
 * test. Nothing is written until `save`.
 * @param {string} name
 * @param {string} text
 * @returns {Promise<EditorStateView>}
 */
export function openText(name, text) {
  return invoke('editor_open_text', { name, text });
}

/**
 * @typedef {Object} EditorUpdate
 * @property {number} revision
 * @property {{ anchor: EditorPosition, head: EditorPosition }} selection
 * @property {string[]} activeMarks
 * @property {number} wordCount
 * @property {boolean} canUndo
 * @property {boolean} canRedo
 * @property {boolean} structural - True when `blocks` is the whole document
 *   again, because its structure moved; false when it is only what changed.
 * @property {EditorBlock[]} blocks
 * @property {number} engineMicros - How long Rust took, by its own clock.
 *   Everything else a keystroke costs is the bridge and the view.
 * @property {{ id: number, level: number, text: string }[]} [outline] - Only
 *   sent with a structural change.
 */

/**
 * Applies commands in order and returns **what changed**, not the whole
 * document: typing touches one block, and sending 800 of them per keystroke is
 * what makes a round trip expensive. A batch means the same thing as the same
 * commands sent one at a time, so a keystroke and its selection change can
 * travel together.
 *
 * Use {@link patchState} to fold the update into the state from `openDocument`.
 * @param {string} name
 * @param {EditorCommand | EditorCommand[]} commands
 * @returns {Promise<EditorUpdate>}
 */
export function apply(name, commands) {
  return invoke('editor_apply', {
    name,
    commands: Array.isArray(commands) ? commands : [commands],
  });
}

/**
 * The current state, without changing anything.
 * @param {string} name
 * @returns {Promise<EditorStateView>}
 */
export function getState(name) {
  return invoke('editor_state', { name });
}

/**
 * The Markdown that a save would write, front matter included.
 * @param {string} name
 * @returns {Promise<string>}
 */
export function getMarkdown(name) {
  return invoke('editor_markdown', { name });
}

/**
 * Writes the document, through the same path the existing save uses: one file
 * write, one version in the undo tree, one file-watcher event for the indexer.
 * @param {string} name
 * @returns {Promise<{ name: string, path: string, last_modified: number, folder: string | null }>}
 */
export function save(name) {
  return invoke('editor_save', { name });
}

/**
 * Forgets the session. Unsaved changes are lost, so save first.
 * @param {string} name
 * @returns {Promise<void>}
 */
export function close(name) {
  return invoke('editor_close', { name });
}

/** Convenience wrappers for the commands with awkward shapes. */
export const commands = {
  /** @param {string} text @returns {EditorCommand} */
  insertText: (text) => ({ command: 'insertText', text }),
  /** @param {EditorPosition} anchor @param {EditorPosition} [head] @returns {EditorCommand} */
  select: (anchor, head) => ({ command: 'setSelection', anchor, head: head ?? anchor }),
  /** @param {boolean} [forward] @returns {EditorCommand} */
  delete: (forward = false) => ({ command: 'delete', forward }),
  /** @param {'bold' | 'italic' | 'code' | 'strike'} mark @returns {EditorCommand} */
  toggleMark: (mark) => ({ command: 'toggleMark', mark }),
  /** @param {number} level @returns {EditorCommand} */
  heading: (level) => ({ command: 'setHeadingLevel', level }),
  /** @param {string | null} url @returns {EditorCommand} */
  link: (url) => ({ command: 'setLink', url }),
};

/**
 * Folds an update into a document state, in place. Only the blocks the update
 * carries are replaced, so a view watching this state redraws one paragraph
 * rather than the document.
 * @param {EditorStateView} state - Mutated.
 * @param {EditorUpdate} update
 * @returns {EditorStateView} The same state, for convenience.
 */
export function patchState(state, update) {
  state.revision = update.revision;
  state.selection = update.selection;
  state.activeMarks = update.activeMarks;
  state.wordCount = update.wordCount;
  state.canUndo = update.canUndo;
  state.canRedo = update.canRedo;
  if (update.outline) state.outline = update.outline;

  if (update.structural) {
    state.blocks = update.blocks;
    return state;
  }
  for (const block of update.blocks) replaceBlock(state.blocks, block);
  return state;
}

/**
 * Puts `next` where the block with its id sits, at any depth.
 * @param {EditorBlock[]} blocks
 * @param {EditorBlock} next
 * @returns {boolean} Whether it was found.
 */
function replaceBlock(blocks, next) {
  for (let index = 0; index < blocks.length; index += 1) {
    const block = blocks[index];
    if (block.id === next.id) {
      blocks[index] = next;
      return true;
    }
    const nested = [
      block.children ?? [],
      ...(block.items ?? []).map((item) => item.blocks),
      ...(block.rows ?? []).flatMap((row) => row.cells),
    ];
    for (const list of nested) {
      if (replaceBlock(list, next)) return true;
    }
  }
  return false;
}

/**
 * The plain text of a block, for measuring or for a preview. Runs that carry no
 * text (an image, a break) contribute nothing.
 * @param {EditorBlock} block
 * @returns {string}
 */
export function blockText(block) {
  if (block.runs?.length) return block.runs.map((run) => run.text ?? '').join('');
  return block.text ?? '';
}
