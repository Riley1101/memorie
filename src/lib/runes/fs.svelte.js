import { invoke } from "@tauri-apps/api/core";
import { join } from "@tauri-apps/api/path";

/**
 * @typedef {Object} FileEntry
 * @property {string} name - The display name of the file.
 * @property {string} path - The full, absolute path to the file.
 */

const rs_commands = {
  /**
   * @returns {Promise<FileEntry[]>}
   */
  getFiles: async () => {
    return await invoke("list_files");
  },

  /**
   * @returns {Promise<FileEntry[]>}
   */
  getRecents: async () => {
    return await invoke("list_recents");
  },
  /**
   * @param {string} name - The name of the file to read.
   * @returns {Promise<string>}
   */
  readFile: async (name) => {
    return await invoke("read_file", { name });
  },
  /**
   * @param {string} path - The path of the file to save.
   * @param {string} content - The content to write to the file.
   * @returns {Promise<string>}
   */
  saveFile: async (path, content) => {
    return await invoke("save_file", { path, content });
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
  currentContent = $state("");

  /**
   * A flag to indicate when an async operation is in progress.
   * @type {boolean}
   */
  isLoading = $state(false);

  /**
   * Holds the message of the last error that occurred.
   * @type {string}
   */
  errorMessage = $state("");


  /**
   * Fetches the list of all .md files from the backend.
   * @async
   * @returns {Promise<void>}
   */
  async getFiles() {
    this.isLoading = true;
    this.errorMessage = "";
    try {
      this.files = await rs_commands.getFiles();
    } catch (err) {
      console.error(err);
      this.errorMessage = `Failed to discover files: ${err}`;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Fetches the list of recent files from the backend.
   * @async
   * @returns {Promise<void>}
   */
  async getRecents() {
    this.isLoading = true;
    this.errorMessage = "";
    try {
        this.files = await rs_commands.getRecents();
    } catch (err) {
      console.error(err);
      this.errorMessage = `Failed to discover recent files: ${err}`;
    }finally {
      this.isLoading = false;
    }
  }

  /**
   * Creates a new, empty .md file and refreshes the file list.
   * @async
   * @param {string} fileName - The name for the new file (without extension).
   * @param {string} [content=""] - Optional initial content for the new file.
   * @returns {Promise<void>}
   */
  async createNewFile(fileName, content = "") {
    if (!fileName || !fileName.trim()) {
      this.errorMessage = "File name cannot be empty.";
      return;
    }

    const finalFileName = fileName.endsWith(".md")
      ? fileName
      : `${fileName}.md`;

    this.isLoading = true;
    this.errorMessage = "";

    try {
      await invoke("create_file", { name : finalFileName, content });
      await this.getFiles();
    } catch (err) {
      this.errorMessage = `Failed to create file "${finalFileName}": ${err}`;
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
