import { editorState } from '$lib/runes/editor.svelte';
import { invoke } from '@tauri-apps/api/core';

/** @typedef {import('$lib/runes/editor.svelte.js').ChunkItem} ChunkItem */

/**
 * Synchronize grammar checks for the given file.
 * @param fileName {string} - The name of the file to synchronize grammar checks for.
 * @returns {Promise<ChunkItem[]>} - A promise that resolves to an array of ChunkItem objects.
 */
export async function syncGrammarChecks(fileName) {
  /** @type {{ data?: ChunkItem[] }} */
  const chunks = await invoke('get_document_context', { name: fileName });
  const items = chunks?.data || [];
  editorState.setGrammarChecks(items);
  return items;
}
