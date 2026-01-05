import { editorState } from '$lib/runes/editor.svelte';
import { invoke } from '@tauri-apps/api/core';

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

/**
 * Synchronize grammar checks for the given file.
 * @param fileName {string} - The name of the file to synchronize grammar checks for.
 * @returns {Promise<ChunkItem[]>} - A promise that resolves to an array of ChunkItem objects.
 */
export async function syncGrammarChecks(fileName) {
  invoke('get_document_context', { name: fileName }).then((chunks) => {
    editorState.setGrammarChecks(chunks.data || []);
    return chunks?.data || [];
  });
}
