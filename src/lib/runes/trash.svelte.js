import { invoke } from '@tauri-apps/api/core';
import { toast } from '$lib/toast.js';
import { fileManager, baseOf } from '$lib/runes/fs.svelte.js';

/**
 * @typedef {Object} TrashEntry
 * @property {string} id
 * @property {string} original_path - Content-relative path it had when deleted.
 * @property {'file' | 'folder'} kind
 * @property {number} deleted_at - Unix seconds.
 * @property {number} item_count - Writings inside, for a folder; 1 for a writing.
 */

/** Matches `RETENTION_DAYS` in the Rust side; shown so the promise is explicit. */
export const RETENTION_DAYS = 30;

/** Strip ".md" for display. */
function label(path) {
  return baseOf(path).replace(/\.md$/i, '');
}

/**
 * Deleted writings and folders, and putting them back.
 *
 * Deleting is the one action in the app that used to be unrecoverable, so
 * every delete now goes here first and every delete toast offers an undo.
 */
class TrashManager {
  /** @type {TrashEntry[]} */
  entries = $state([]);
  isLoading = $state(false);

  /** True once a listing has come back, so the UI can tell empty from unloaded. */
  hasLoaded = $state(false);

  async refresh() {
    this.isLoading = true;
    try {
      this.entries = await invoke('list_trash');
    } catch (err) {
      console.error(err);
      toast.error('Could not read the trash', err);
    } finally {
      this.isLoading = false;
      this.hasLoaded = true;
    }
  }

  /**
   * Puts an item back and refreshes the listings it affects.
   * @param {string} id
   * @returns {Promise<string|null>} The path it landed at, or null on failure.
   */
  async restore(id) {
    try {
      /** @type {string} */
      const restored = await invoke('restore_from_trash', { id });
      await Promise.all([
        fileManager.getRecents(),
        fileManager.getFolders(),
        fileManager.getBinders(),
        this.refresh(),
      ]);
      return restored;
    } catch (err) {
      console.error(err);
      toast.error('Could not put that back', err);
      return null;
    }
  }

  /**
   * Restores and says so, including when the old name was taken in the
   * meantime and the writing came back under a new one.
   * @param {TrashEntry} entry
   */
  async restoreWithToast(entry) {
    const restored = await this.restore(entry.id);
    if (!restored) return null;
    const renamed = restored !== entry.original_path;
    toast.success(
      `Put “${label(entry.original_path)}” back`,
      renamed ? `The old name was taken, so it came back as “${label(restored)}”.` : undefined
    );
    return restored;
  }

  /** @param {string} id */
  async purge(id) {
    try {
      await invoke('purge_from_trash', { id });
      await this.refresh();
      return true;
    } catch (err) {
      console.error(err);
      toast.error('Could not delete that for good', err);
      return false;
    }
  }

  async empty() {
    try {
      /** @type {number} */
      const removed = await invoke('empty_trash');
      await this.refresh();
      toast.success(removed === 1 ? 'Trash emptied' : `Trash emptied — ${removed} items removed`);
      return true;
    } catch (err) {
      console.error(err);
      toast.error('Could not empty the trash', err);
      return false;
    }
  }
}

export const trashManager = new TrashManager();

/**
 * The toast shown after a delete: says what went, and offers it straight back.
 * @param {TrashEntry} entry
 */
export function offerUndo(entry) {
  const what = label(entry.original_path);
  const description =
    entry.kind === 'folder'
      ? `${entry.item_count} writing${entry.item_count === 1 ? '' : 's'} moved to the trash.`
      : `In the trash for ${RETENTION_DAYS} days.`;
  toast.successWithAction(
    `Deleted “${what}”`,
    'Undo',
    () => {
      trashManager.restoreWithToast(entry);
    },
    description
  );
}
