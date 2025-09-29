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
    return await invoke("discover_files");
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

  // --- METHODS ---

  /**
   * Fetches the list of all .lexical files from the backend.
   * @async
   * @returns {Promise<void>}
   */
  async getFiles() {
    this.isLoading = true;
    this.errorMessage = "";
    try {
      this.files = await rs_commands.getFiles();
    } catch (err) {
      this.errorMessage = `Failed to discover files: ${err}`;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Selects a file, fetching its content and updating the state.
   * @async
   * @param {FileEntry | null} file - The file to select.
   * @returns {Promise<void>}
   */
  async readFile(file) {
    if (!file) {
      this.currentFile = null;
      this.currentContent = "";
      return;
    }

    this.isLoading = true;
    this.errorMessage = "";
    try {
      const content = await rs_commands.readFile(file.name);
      this.currentFile = file;
      this.currentContent = content;
    } catch (err) {
      this.errorMessage = `Failed to read file "${file.name}": ${err}`;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Saves the content of the currently selected file to the backend.
   * @async
   * @param {string} name - The name of the file to save.
   * @param {string} content - The content to save.
   * @returns {Promise<void>}
   */
  async saveCurrentFile(name, content) {
    if (!this.currentFile) {
      this.errorMessage = "No file is selected to save.";
      return;
    }

    this.isLoading = true;
    this.errorMessage = "";
    try {
      await invoke("update_file", {
        name: name,
        content: content,
      });
    } catch (err) {
      this.errorMessage = `Failed to save file "${this.currentFile.name}": ${err}`;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Creates a new, empty .lexical file and refreshes the file list.
   * @async
   * @param {string} fileName - The name for the new file (without extension).
   * @returns {Promise<void>}
   */
  async createNewFile(fileName) {
    if (!fileName || !fileName.trim()) {
      this.errorMessage = "File name cannot be empty.";
      return;
    }

    const finalFileName = fileName.endsWith(".lexical")
      ? fileName
      : `${fileName}.lexical`;

    this.isLoading = true;
    this.errorMessage = "";
    try {
      const contentDir = await invoke("get_content_directory");
      const newFilePath = await join(contentDir, finalFileName);

      await invoke("save_file", { path: newFilePath, content: "" });

      // Refresh the file list to show the new file
      await this.getFiles();
    } catch (err) {
      this.errorMessage = `Failed to create file "${finalFileName}": ${err}`;
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
