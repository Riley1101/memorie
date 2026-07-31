import { invoke } from '@tauri-apps/api/core';

/**
 * @typedef {Object} FileEntry
 * @property {string} name - The file's identifier, relative to the content directory (e.g. "note.md" or "Binder/note.md").
 * @property {string} path - The full, absolute path to the file.
 * @property {number} last_modified - The unix timestamp of when the file was last modified.
 * @property {string|null} folder - The binder (top-level folder) name the file lives in, or null.
 */

const rs_commands = {
  /**
   * @returns {Promise<FileEntry[]>}
   */
  getRecents: async () => {
    return await invoke('list_recents');
  },
  /**
   * @returns {Promise<string[]>}
   */
  getBinders: async () => {
    return await invoke('list_binders');
  },
};

/**
 * Manages file state and interactions with the Tauri backend.
 * NOTE: This class uses Svelte 5 runes (`$state`) and must be used
 * in a Svelte 5 environment to be reactive.
 * @class
 */
class FileManager {
  // --- STATE ---

  /**
   * The list of all discovered files.
   * @type {FileEntry[]}
   */
  files = $state([]);

  /**
   * The currently selected file object.
   * @type {FileEntry | null}
   */
  currentFile = $state(null);

  /**
   * The text content of the currently selected file.
   * @type {string}
   */
  currentContent = $state('');

  /**
   * The list of all discovered binders (top-level folders).
   * @type {string[]}
   */
  binders = $state([]);

  /**
   * A flag to indicate when an async operation is in progress.
   * @type {boolean}
   */
  isLoading = $state(false);

  /**
   * Holds the message of the last error that occurred.
   * @type {string}
   */
  errorMessage = $state('');

  /**
   * Fetches the list of recent files from the backend.
   * @async
   * @returns {Promise<void>}
   */
  async getRecents() {
    this.isLoading = true;
    this.errorMessage = '';
    try {
      this.files = await rs_commands.getRecents();
    } catch (err) {
      console.error(err);
      this.errorMessage = `Failed to discover recent files: ${err}`;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Fetches the list of binders (top-level folders) from the backend.
   * @async
   * @returns {Promise<void>}
   */
  async getBinders() {
    try {
      this.binders = await rs_commands.getBinders();
    } catch (err) {
      console.error(err);
      this.errorMessage = `Failed to discover binders: ${err}`;
    }
  }

  /**
   * Creates a new binder (folder) and refreshes the binder list.
   * @async
   * @param {string} binderName - The name for the new binder.
   * @returns {Promise<void>}
   */
  async createBinder(binderName) {
    if (!binderName || !binderName.trim()) {
      this.errorMessage = 'Binder name cannot be empty.';
      return;
    }

    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('create_binder', { name: binderName.trim() });
      await this.getBinders();
    } catch (err) {
      this.errorMessage = `Failed to create binder "${binderName}": ${err}`;
      console.error(err);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Creates a new, empty .md file and refreshes the file list.
   * @async
   * @param {string} fileName - The name for the new file (without extension).
   * @param {string} [content=""] - Optional initial content for the new file.
   * @param {string|null} [binder=null] - Optional binder (folder) to create the file in.
   * @returns {Promise<void>}
   */
  async createNewFile(fileName, content = '', binder = null) {
    if (!fileName || !fileName.trim()) {
      this.errorMessage = 'File name cannot be empty.';
      return;
    }

    const baseName = fileName.endsWith('.md') ? fileName : `${fileName}.md`;
    const finalFileName = binder ? `${binder}/${baseName}` : baseName;

    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('create_file', { name: finalFileName, content });
      await this.getRecents();
    } catch (err) {
      this.errorMessage = `Failed to create file "${finalFileName}": ${err}`;
      console.error(err);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Undoes the last change made to the specified file.
   * @async
   * @param {string} fileName - The name of the file to undo the last change for.
   * @returns {Promise<void>}
   */
  async undoFile(fileName) {
    this.isLoading = true;
    this.errorMessage = '';

    try {
      const result = await invoke('undo_file', { name: fileName });
      console.log(`Undo result for file "${fileName}":`, result);
    } catch (err) {
      this.errorMessage = `Failed to undo last change for file "${fileName}": ${err}`;
      console.error(err);
    }
  }

  /**
   * Redoes the last undone change for the specified file.
   * @async
   * @param {string} fileName - The name of the file to redo the last undone change for.
   * @returns {Promise<void>}
   */
  async redoFile(fileName) {
    this.isLoading = true;
    this.errorMessage = '';

    try {
      const result = await invoke('redo_file', { name: fileName });
      console.log(`Redo result for file "${fileName}":`, result);
    } catch (err) {
      this.errorMessage = `Failed to redo last undone change for file "${fileName}": ${err}`;
      console.error(err);
    }
  }

  /**
   * Deletes a file and refreshes the file list.
   * @async
   * @param {string} fileName - The name of the file to delete.
   * @returns {Promise<void>}
   */
  async deleteFile(fileName) {
    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('delete_file', { name: fileName });
      await this.getRecents();
    } catch (err) {
      this.errorMessage = `Failed to delete file "${fileName}": ${err}`;
      console.error(err);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Renames a file and refreshes the file list.
   * @async
   * @param {string} oldName - The current name of the file.
   * @param {string} newName - The new name for the file.
   * @returns {Promise<void>}
   */
  async renameFile(oldName, newName) {
    if (!newName || !newName.trim()) {
      this.errorMessage = 'New file name cannot be empty.';
      return;
    }

    const finalNewName = newName.endsWith('.md') ? newName : `${newName}.md`;
    if (oldName === finalNewName) return;

    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('rename_file', { oldName, newName: finalNewName });
      await this.getRecents();
    } catch (err) {
      this.errorMessage = `Failed to rename file from "${oldName}" to "${finalNewName}": ${err}`;
      console.error(err);
    } finally {
      this.isLoading = false;
    }
  }
}

/**
 * A singleton instance of the FileManager for the entire application to use.
 * @example
 * import { fileManager } from "./fs.svelte.js";
 *
 * fileManager.createNewFile("new_file");
 */
export const fileManager = new FileManager();
