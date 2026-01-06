import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/**
 * LLM events
 * @type {Record<string, string>}
 *
 * This enum map directly to ChatEvent enum in Rust backend
 * @see src-tauri/workers.rs
 */
const LLM_EVENTS = {
  CHAT_INIT: 'chat-init',
  CHAT_IN_PROGRESS: 'chat-in-progress',
  AUTOCOMPOLETE: 'chat-autocomplete',
  COMPLETED: 'chat-completed',
  Error: 'chat-error',

  EDIT_ACTION_START: 'chat-edit-action-start',
  EDIT_ACTION_IN_PROGRESS: 'chat-edit-action-in-progress',
  EDIT_ACTION_COMPLETED: 'chat-edit-action-completed',
};

const LLM_INVOKE = {
  CHAT: 'run_chat',
  CANCEL_CHAT: 'cancel_chat',
};

/**
 * @typedef {'user' | 'assistant'} MessageRole
 */

/**
 * @typedef {object} Message
 * @property {MessageRole} role - The role of the message sender.
 * @property {string} content - The text content of the message.
 */

/**
 * Manages the state and communication for an LLM chat interface in a Tauri app.
 */
export class LlmManager {
  /** @type string | null - unique worker ID **/
  workerId = $state(null);

  /** @type string | null - Edit Action Type **/
  editActionType = $state(null);

  /** @type string | null - Edit Action Response **/
  editActionContent = $state(null);

  /** @type boolean - indicates if edit action is in progress **/
  editActionInProgress = $state(false);

  /** @type boolean - indicates if models are loaded **/
  modelsLoaded = $state(false);

  /** @type Message[] - array of messages **/
  messages = $state([]);

  /** @type boolean - loading state **/
  isLoading = $state(false);

  /** @type string | null - error message **/
  error = $state(null);

  /** @type Function | null - function to unlisten chunk events **/
  unlistenChunk = null;

  /** @type Function | null - function to unlisten chunk events **/
  unlistenDone = null;

  /**
   * Sets up Tauri event listeners to receive streaming data from the Rust backend.
   */
  async setupModels() {
    this.modelsLoaded = false;

    await invoke('load_models').then(() => {
      this.modelsLoaded = true;
    });

    await listen(LLM_EVENTS.CHAT_INIT, (response) => {
      if (response.event === 'chat-in-progress') {
        this.isLoading = true;
      }
    });

    await listen(LLM_EVENTS.EDIT_ACTION_START, (response) => {
      if (response.event === 'chat-edit-action-start') {
        this.editActionInProgress = true;
        this.editActionContent = '';
      }
    });

    await listen(LLM_EVENTS.AUTOCOMPOLETE, (event) => {
      const chunk = /** @type {string} */ (event.payload.response);
      if (this.messages.length > 0) {
        const lastMessage = this.messages[this.messages.length - 1];
        if (lastMessage.role === 'assistant') {
          lastMessage.content += JSON.stringify(chunk);
        }
      }
    });

    this.unlistenChunk = await listen(LLM_EVENTS.CHAT_IN_PROGRESS, (event) => {
      const chunk = /** @type {string} */ (event.payload.content);
      if (this.messages.length > 0) {
        const lastMessage = this.messages[this.messages.length - 1];
        if (lastMessage.role === 'assistant') {
          lastMessage.content += chunk;
        }
      }
    });

    this.unlistenChunk = await listen(LLM_EVENTS.EDIT_ACTION_IN_PROGRESS, (event) => {
      const chunk = /** @type {string} */ (event.payload.content);
      this.editActionContent = this.editActionContent + chunk;
    });

    // Listener for when the stream is complete
    this.unlistenDone = await listen(LLM_EVENTS.COMPLETED, (response) => {
      if (response.event === 'chat-completed') {
        this.isLoading = false;
      }
    });

    // Listener for when the edit action is complete
    this.unlistenDone = await listen(LLM_EVENTS.EDIT_ACTION_COMPLETED, (response) => {
      if (response.event === 'chat-edit-action-completed') {
        this.editActionInProgress = false;
      }
    });
  }

  /**
   * Sends a user's prompt to the Rust backend to start the LLM stream.
   * @param {string} prompt The user's message.
   * @param {string} [mode] The mode of the chat (default is "Normal").
   * @returns {Promise<void>}
   */
  async sendMessage(prompt, mode = 'Normal') {
    if (this.isLoading || !prompt.trim()) {
      return;
    }
    this.isLoading = true;
    this.error = null;

    this.messages.push({ role: 'user', content: prompt });
    this.messages.push({ role: 'assistant', content: '' });

    try {
      /** @type {string} processId */
      this.workerId = await invoke(LLM_INVOKE.CHAT, {
        message: prompt,
        mode,
      });
    } catch (e) {
      console.error(e);
      this.error = `An error occurred: ${e}`;
      this.isLoading = false;
      this.messages.pop();
    }
  }

  /**
   * Sends an edit action instruction to the Rust backend to modify an existing message.
   * @param {string} originalMessage The original message to be edited.
   * @param {string} editInstruction The instruction for editing the message.
   * @returns {Promise<void>}
   */
  async sendEditActionMessage(originalMessage, editInstruction) {
    if (this.editActionInProgress || !editInstruction.trim()) {
      return;
    }
    this.editActionType = editInstruction;
    this.editActionInProgress = true;

    try {
      /** @type {string} processId */
      this.workerId = await invoke(LLM_INVOKE.CHAT, {
        message: originalMessage,
        editAction: editInstruction,
        mode: 'EditAction',
      });
    } catch (e) {
      console.error(e);
      this.error = `An error occurred: ${e}`;
      this.editActionInProgress = false;
    }
  }

  /**
   * Cancels the ongoing LLM message generation.
   * @returns {Promise<void>}
   */
  async cancelMessage() {
    if (this.workerId) {
      let status = await invoke(LLM_INVOKE.CANCEL_CHAT, { jobId: this.workerId });
      if (status) {
        this.isLoading = false;
        this.workerId = null;
        this.messages[this.messages.length - 1].content += '\n\nCancelled by user.';
      }
    }
  }

  /**
   * @public
   *  Clear message array and start a new chart
   */
  newSession() {
    this.messages = [];
  }

  /**
   * @public
   * Clear edit action state and start a new edit action session
   */
  newEditActionSession() {
    this.editActionType = null;
    this.editActionContent = null;
  }

  /**
   * Cleans up the event listeners. Call this when the component is destroyed.
   */
  destroy() {
    if (this.unlistenChunk) {
      this.unlistenChunk();
    }
    if (this.unlistenDone) {
      this.unlistenDone();
    }
  }
}

export const llmManager = new LlmManager();
