import { invoke } from '@tauri-apps/api/core';
import { toast } from '$lib/toast.js';

/**
 * Directory part of a content-relative name, e.g. "Novel/Ch 1/Scene.md" -> "Novel/Ch 1".
 * @param {string} name
 * @returns {string}
 */
export function dirOf(name) {
  const i = name.lastIndexOf('/');
  return i === -1 ? '' : name.slice(0, i);
}

/**
 * Basename of a content-relative name, e.g. "Novel/Ch 1/Scene.md" -> "Scene.md".
 * @param {string} name
 * @returns {string}
 */
export function baseOf(name) {
  return name.slice(name.lastIndexOf('/') + 1);
}

/** Basename without the ".md" extension, for messages. */
function formatBase(base) {
  return base.replace(/\.md$/i, '');
}

/**
 * @typedef {Object} FileEntry
 * @property {string} name - The file's identifier, relative to the content directory (e.g. "note.md", "Binder/note.md", or "Binder/Chapter 1/Scene 1.md").
 * @property {string} path - The full, absolute path to the file.
 * @property {number} last_modified - The unix timestamp of when the file was last modified.
 * @property {string|null} folder - The binder (top-level folder) name the file lives in, or null. Files can be nested further (chapters, scenes) below the binder; `name` carries the full path.
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
  /**
   * @returns {Promise<string[]>}
   */
  getFolders: async () => {
    return await invoke('list_folders');
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
   * Every directory (binder, chapter, scene grouping, ...) under the content
   * directory, as `/`-joined relative paths. Lets folders with no writings in
   * them yet still show up in a binder's tree.
   * @type {string[]}
   */
  folders = $state([]);

  /**
   * A flag to indicate when an async operation is in progress.
   * @type {boolean}
   */
  isLoading = $state(false);

  /**
   * True once the first file listing has come back from the backend. Lets the
   * UI show a skeleton instead of flashing the empty state on launch.
   * @type {boolean}
   */
  hasLoadedFiles = $state(false);

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
      toast.error('Could not load writings', err);
    } finally {
      this.isLoading = false;
      this.hasLoadedFiles = true;
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
   * Fetches the list of all folders (including nested, empty ones) from the backend.
   * @async
   * @returns {Promise<void>}
   */
  async getFolders() {
    try {
      this.folders = await rs_commands.getFolders();
    } catch (err) {
      console.error(err);
      this.errorMessage = `Failed to discover folders: ${err}`;
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
      await this.getFolders();
    } catch (err) {
      this.errorMessage = `Failed to create binder "${binderName}": ${err}`;
      console.error(err);
      toast.error(`Could not create binder "${binderName}"`, err);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Renames a binder (folder) and refreshes the binder list.
   * @async
   * @param {string} oldName - The current binder name.
   * @param {string} newName - The new binder name.
   * @returns {Promise<void>}
   */
  async renameBinder(oldName, newName) {
    if (!newName || !newName.trim() || oldName === newName.trim()) return;

    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('rename_binder', { oldName, newName: newName.trim() });
      await this.getBinders();
      await this.getRecents();
      await this.getFolders();
    } catch (err) {
      this.errorMessage = `Failed to rename binder "${oldName}": ${err}`;
      console.error(err);
      toast.error(`Could not rename "${oldName}"`, err);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Deletes an empty binder (folder) and refreshes the binder list.
   * Refuses (backend-enforced) if the binder still contains writings.
   * @async
   * @param {string} binderName - The name of the binder to delete.
   * @returns {Promise<void>}
   */
  async deleteBinder(binderName) {
    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('delete_binder', { name: binderName });
      await this.getBinders();
      await this.getFolders();
    } catch (err) {
      this.errorMessage = `Failed to delete binder "${binderName}": ${err}`;
      console.error(err);
      toast.error(`Could not delete binder "${binderName}"`, err);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Creates a new, empty .md file and refreshes the file list.
   * @async
   * @param {string} fileName - The name for the new file (without extension).
   * @param {string} [content=""] - Optional initial content for the new file.
   * @param {string|null} [binder=null] - Optional binder (top-level folder) to create the file in.
   * @param {string[]} [subPath=[]] - Optional chain of folder names nested inside the binder (e.g. ["Chapter 1"]).
   * @returns {Promise<string|null>} The full content-relative name of the created file, or null on failure.
   */
  async createNewFile(fileName, content = '', binder = null, subPath = []) {
    if (!fileName || !fileName.trim()) {
      this.errorMessage = 'File name cannot be empty.';
      return null;
    }

    const baseName = fileName.endsWith('.md') ? fileName : `${fileName}.md`;
    const segments = binder ? [binder, ...subPath, baseName] : [baseName];
    const finalFileName = segments.join('/');

    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('create_file', { name: finalFileName, content });
      await this.getRecents();
      if (subPath.length) await this.getFolders();
      return finalFileName;
    } catch (err) {
      this.errorMessage = `Failed to create file "${finalFileName}": ${err}`;
      console.error(err);
      toast.error(`Could not create "${baseOf(finalFileName)}"`, err);
      return null;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Picks an unused "Untitled" name (Untitled, Untitled 2, ...) in the given location.
   * @param {string|null} [binder=null]
   * @param {string[]} [subPath=[]]
   * @returns {string} Name without extension.
   */
  untitledName(binder = null, subPath = []) {
    const dir = binder ? [binder, ...subPath].join('/') : '';
    return this.uniqueName(dir, 'Untitled');
  }

  /**
   * Returns `base` (no extension) unchanged if free in `dir`, otherwise
   * "base 2", "base 3", ... The check is against the last listing we have.
   * @param {string} dir - Content-relative folder ('' for top level).
   * @param {string} base - Desired name without ".md".
   * @returns {string}
   */
  uniqueName(dir, base) {
    const taken = new Set(
      this.files.filter((f) => dirOf(f.name) === dir).map((f) => baseOf(f.name).toLowerCase())
    );
    let n = 1;
    let candidate = base;
    while (taken.has(`${candidate}.md`.toLowerCase())) {
      n += 1;
      candidate = `${base} ${n}`;
    }
    return candidate;
  }

  /**
   * Creates a file at an exact content-relative path. Throws on failure so the
   * editor can surface it; the caller decides how to name the file.
   * @param {string} fullName - e.g. "Novel/Chapter 1/Scene 1.md".
   * @param {string} content
   * @returns {Promise<void>}
   */
  async createFileAt(fullName, content) {
    await invoke('create_file', { name: fullName, content });
    await this.getRecents();
    if (dirOf(fullName)) await this.getFolders();
  }

  /**
   * Writes the current content of an existing file (autosave). Unlike
   * createNewFile this throws on failure so the editor can show the real
   * save status.
   * @async
   * @param {string} fileName - Full content-relative name.
   * @param {string} content
   * @returns {Promise<void>}
   */
  async saveFile(fileName, content) {
    await invoke('create_file', { name: fileName, content });
    await this.getRecents();
  }

  /**
   * Creates an empty nested folder (e.g. a chapter or scene grouping) inside a
   * binder and refreshes the file list.
   * @async
   * @param {string} binder - The binder (top-level folder) the new folder lives under.
   * @param {string[]} subPath - Chain of existing folder names the new folder nests under (e.g. ["Chapter 1"]).
   * @param {string} folderName - Name of the new folder (e.g. "Chapter 2" or "Scene 1").
   * @returns {Promise<void>}
   */
  async createFolder(binder, subPath, folderName) {
    if (!folderName || !folderName.trim()) {
      this.errorMessage = 'Folder name cannot be empty.';
      return;
    }

    const relativePath = [binder, ...subPath, folderName.trim()].filter(Boolean).join('/');

    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('create_folder', { relativePath });
      await this.getFolders();
    } catch (err) {
      this.errorMessage = `Failed to create folder "${relativePath}": ${err}`;
      console.error(err);
      toast.error(`Could not create folder "${folderName.trim()}"`, err);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Deletes an empty nested folder (chapter, scene grouping, ...).
   * Refuses (backend-enforced) if the folder still contains anything.
   * @async
   * @param {string} relativePath - `/`-joined path of the folder to delete (e.g. "Novel/Chapter 1").
   * @returns {Promise<void>}
   */
  async deleteFolder(relativePath) {
    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('delete_folder', { relativePath });
      await this.getFolders();
    } catch (err) {
      this.errorMessage = `Failed to delete folder "${relativePath}": ${err}`;
      console.error(err);
      toast.error(`Could not delete folder "${baseOf(relativePath)}"`, err);
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
      await invoke('undo_file', { name: fileName });
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
      await invoke('redo_file', { name: fileName });
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
      toast.error(`Could not delete "${baseOf(fileName)}"`, err);
    } finally {
      this.isLoading = false;
    }
  }

  /** Files whose content-relative name sits anywhere under `dir`. */
  filesUnder(dir) {
    const prefix = `${dir}/`;
    return this.files.filter((f) => f.name.startsWith(prefix));
  }

  /** Folders (as `/`-joined paths) anywhere under `dir`, excluding `dir` itself. */
  foldersUnder(dir) {
    const prefix = `${dir}/`;
    return this.folders.filter((f) => f.startsWith(prefix));
  }

  /**
   * Moves a writing into another folder (or the top level), keeping its name.
   * @param {string} oldName - Full content-relative name.
   * @param {string} newDir - Destination folder ('' for top level).
   * @returns {Promise<string|null>} New full name, or null if nothing moved.
   */
  async moveFile(oldName, newDir) {
    const base = baseOf(oldName);
    const newName = newDir ? `${newDir}/${base}` : base;
    if (newName === oldName) return null;

    const clash = this.files.some((f) => f.name.toLowerCase() === newName.toLowerCase());
    if (clash) {
      toast.error(`"${formatBase(base)}" already exists there`, 'Rename one of them first.');
      return null;
    }

    this.isLoading = true;
    this.errorMessage = '';
    try {
      await invoke('move_file', { oldName, newName });
      await this.getRecents();
      await this.getFolders();
      return newName;
    } catch (err) {
      this.errorMessage = `Failed to move "${oldName}": ${err}`;
      console.error(err);
      toast.error(`Could not move "${formatBase(base)}"`, err);
      return null;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Moves a folder (with everything in it) under another folder or binder.
   * @param {string} oldPath - e.g. "Novel/Part 1/Chapter 3".
   * @param {string} newParent - Destination folder or binder, e.g. "Novel/Part 2".
   * @returns {Promise<string|null>} New path, or null if nothing moved.
   */
  async moveFolder(oldPath, newParent) {
    const base = baseOf(oldPath);
    const newPath = newParent ? `${newParent}/${base}` : base;
    if (newPath === oldPath) return null;
    if (newParent === oldPath || newParent.startsWith(`${oldPath}/`)) {
      toast.error('Cannot move a folder inside itself');
      return null;
    }
    if (!newParent) {
      toast.error('Folders live inside a binder', 'Pick a binder or a folder as the destination.');
      return null;
    }
    const clash = this.folders.some((f) => f.toLowerCase() === newPath.toLowerCase());
    if (clash) {
      toast.error(`A folder named "${base}" already exists there`);
      return null;
    }

    await this.renameBinder(oldPath, newPath);
    return this.folders.includes(newPath) ? newPath : null;
  }

  /**
   * Deletes a folder after moving its direct children (files and folders) up
   * one level. Stops before deleting if any child could not be moved.
   * @param {string} path
   * @returns {Promise<boolean>}
   */
  async liftAndDeleteFolder(path) {
    const parent = dirOf(path);
    const childFiles = this.filesUnder(path).filter((f) => dirOf(f.name) === path);
    const childFolders = this.foldersUnder(path).filter((f) => dirOf(f) === path);

    for (const f of childFiles) {
      const moved = await this.moveFile(f.name, parent);
      if (!moved) return false;
    }
    for (const f of childFolders) {
      const moved = await this.moveFolder(f, parent);
      if (!moved) return false;
    }
    await this.deleteFolder(path);
    return !this.folders.includes(path);
  }

  /**
   * Deletes a folder and everything inside it. Version history for the
   * deleted writings is left untouched on disk.
   * @param {string} path
   * @returns {Promise<boolean>}
   */
  async deleteFolderRecursive(path) {
    this.isLoading = true;
    this.errorMessage = '';
    try {
      for (const f of this.filesUnder(path)) {
        await invoke('delete_file', { name: f.name });
      }
      const nested = this.foldersUnder(path).sort((a, b) => b.length - a.length);
      for (const f of nested) {
        await invoke('delete_folder', { relativePath: f });
      }
      await invoke('delete_folder', { relativePath: path });
      return true;
    } catch (err) {
      this.errorMessage = `Failed to delete folder "${path}": ${err}`;
      console.error(err);
      toast.error(`Could not delete "${baseOf(path)}"`, err);
      return false;
    } finally {
      this.isLoading = false;
      await this.getRecents();
      await this.getFolders();
      await this.getBinders();
    }
  }

  /**
   * Renames a file (keeping it in the same folder) and refreshes the file list.
   * @async
   * @param {string} oldName - The current full content-relative name (e.g. "Novel/Ch 1/Scene.md").
   * @param {string} newName - The new basename, with or without ".md". Slashes are stripped.
   * @returns {Promise<string|null>} The new full name, or null if nothing was renamed.
   */
  async renameFile(oldName, newName) {
    const cleaned = (newName ?? '').trim().replace(/\//g, '');
    if (!cleaned) {
      this.errorMessage = 'New file name cannot be empty.';
      return null;
    }

    const base = cleaned.endsWith('.md') ? cleaned : `${cleaned}.md`;
    const dir = dirOf(oldName);
    const finalNewName = dir ? `${dir}/${base}` : base;
    if (oldName === finalNewName) return null;

    // The OS rename would silently overwrite an existing file.
    const clash = this.files.some(
      (f) => f.name !== oldName && f.name.toLowerCase() === finalNewName.toLowerCase()
    );
    if (clash) {
      this.errorMessage = `A writing named "${base}" already exists here.`;
      toast.error(`"${cleaned}" already exists`, 'Pick a different name.');
      return null;
    }

    this.isLoading = true;
    this.errorMessage = '';

    try {
      await invoke('rename_file', { oldName, newName: finalNewName });
      await this.getRecents();
      return finalNewName;
    } catch (err) {
      this.errorMessage = `Failed to rename file from "${oldName}" to "${finalNewName}": ${err}`;
      console.error(err);
      toast.error(`Could not rename "${baseOf(oldName)}"`, err);
      return null;
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
