/**
 * @typedef MemoryContext {
 *     content: string;
 *     fileName: string;
 * }
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Manages memory-related functionalities for runes.
 */
class MemoryManager {
  /** @type {{content: string, fileName: string}} */
  context = $state({ content: '', fileName: '' });
  setContext(content, fileName) {
    this.context = {
      content,
      fileName,
    };
  }

  /**
   * Gets the current memory context.
   * @returns {{content: string, fileName: string}}
   */
  getContext() {
    return this.context;
  }

  /**
   * Creates embeddings for the current file content.
   * @public
   *
   */
  createDocumentContext() {
    console.log(this.context)
    invoke('create_document_context', {
      content: this.context.content,
      name: this.context.fileName,
    })
      .then((res) => {
        console.log('Embeddings created:', res);
      })
      .catch((err) => {
        console.error('Error creating embeddings:', err);
      });
  }

  /**
   * Searches memory embeddings based on a query.
   *
   * @public
   * @param {string} query - The search query.
   */
  searchMemory(query) {
    invoke('search_embeddings', { query }).catch((err) => {
      console.error('Error searching embeddings:', err);
    });
  }
}

export const memoryManager = new MemoryManager();
