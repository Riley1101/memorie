import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from '$lib/toast.js';

/**
 * @typedef {{ documents: number, passages: number, indexing: boolean }} IndexStatus
 * @typedef {{ done: number, total: number, current: string | null }} IndexProgress
 */

/**
 * Tracks the open document (so the assistant can answer questions about it) and the
 * local note search index (embeddings the assistant searches across all notes).
 */
class MemoryManager {
  /** @type {{content: string, fileName: string}} */
  context = $state({ content: '', fileName: '' });

  /** @type {IndexStatus | null} */
  indexStatus = $state(null);

  /** @type {IndexProgress | null} - set while a reindex pass is running */
  indexProgress = $state(null);

  _listening = false;

  setContext(content, fileName) {
    this.context = {
      content,
      fileName,
    };
  }

  /** The open document as the chat sends it, or null when nothing is open. */
  get document() {
    const { fileName, content } = this.context;
    if (!fileName || !content?.trim()) return null;
    return { title: fileName, content };
  }

  /** Subscribes to indexing events once and loads the current index size. */
  async setup() {
    if (this._listening) return;
    this._listening = true;

    await listen('index-progress', (event) => {
      this.indexProgress = event.payload;
    });
    await listen('index-complete', (event) => {
      this.indexProgress = null;
      this.indexStatus = { ...event.payload, indexing: false };
    });
    await listen('index-error', (event) => {
      this.indexProgress = null;
      toast.error('Could not index notes', event.payload.message);
      this.refreshIndexStatus();
    });

    await this.refreshIndexStatus();
  }

  async refreshIndexStatus() {
    try {
      this.indexStatus = await invoke('get_index_status');
    } catch (err) {
      console.error('Failed to load index status:', err);
    }
  }

  /**
   * Indexes changed notes in the background and prunes deleted ones. Notes unchanged
   * since the last pass aren't read, so this is cheap to run on startup.
   * @param {boolean} [force] - Re-read every note instead of trusting the last pass.
   */
  async reindexNotes(force = false) {
    this.indexProgress ??= { done: 0, total: 0, current: null };
    try {
      await invoke('reindex_notes', { force });
    } catch (err) {
      this.indexProgress = null;
      console.error('Failed to start indexing:', err);
      toast.error('Could not index notes', err);
    }
  }

  /**
   * Re-embeds the open document after a save.
   * @public
   */
  createDocumentContext() {
    invoke('create_document_context', {
      content: this.context.content,
      name: this.context.fileName,
    })
      .then(() => this.refreshIndexStatus())
      .catch((err) => {
        console.error('Error creating context:', err);
      });
  }
}

export const memoryManager = new MemoryManager();
