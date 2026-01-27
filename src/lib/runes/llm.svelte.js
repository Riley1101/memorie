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

  RAG_CHAT_INIT: 'rag-chat-init',
  RAG_CHAT_IN_PROGRESS: 'rag-chat-in-progress',
  RAG_CHAT_COMPLETED: 'rag-chat-completed',
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
 * @property {any[]} [references] - Optional array of referenced articles.
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

  /** @type boolean - loading state **/
  isRAGLoading = $state(false);

  /** @type Message[] - array of RAG Response messages **/
  ragMessages = $state([]);

  /** @type Message[] - array of messages **/
  messages = $state([]);

  /** @type boolean - loading state **/
  isLoading = $state(false);

  /** @type string | null - error message **/
  error = $state(null);

  /** @type {Function[]} - array of unlisten functions **/
  unlisteners = $state([]);

  /**
   * Sets up Tauri event listeners to receive streaming data from the Rust backend.
   */
  async setupModels() {
    this.modelsLoaded = false;

    await invoke('load_models').then(() => {
      this.modelsLoaded = true;
    });

    this.unlisteners.push(
      await listen(LLM_EVENTS.CHAT_INIT, (response) => {
        if (response.event === 'chat-in-progress') {
          this.isLoading = true;
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.RAG_CHAT_INIT, (response) => {
        if (response.event === 'rag-chat-in-progress') {
          this.isRAGLoading = true;
          this.ragMessages.push({ role: 'user', content: response.payload.message });
          this.ragMessages.push({ role: 'assistant', content: '' });
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.EDIT_ACTION_START, (response) => {
        if (response.event === 'chat-edit-action-start') {
          this.editActionInProgress = true;
          this.editActionContent = '';
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.AUTOCOMPOLETE, (event) => {
        const chunk = /** @type {string} */ (event.payload.response);
        if (this.messages.length > 0) {
          const lastMessage = this.messages[this.messages.length - 1];
          if (lastMessage.role === 'assistant') {
            lastMessage.content += chunk;
          }
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.RAG_CHAT_IN_PROGRESS, (event) => {
        const chunk = /** @type {string} */ (event.payload.content);
        if (this.ragMessages.length > 0) {
          const lastMessage = this.ragMessages[this.ragMessages.length - 1];
          if (lastMessage.role === 'assistant') {
            lastMessage.content += chunk;
          }
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.CHAT_IN_PROGRESS, (event) => {
        const chunk = /** @type {string} */ (event.payload.content);
        if (this.messages.length > 0) {
          const lastMessage = this.messages[this.messages.length - 1];
          if (lastMessage.role === 'assistant') {
            lastMessage.content += chunk;
          }
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.EDIT_ACTION_IN_PROGRESS, (event) => {
        const chunk = /** @type {string} */ (event.payload.content);
        this.editActionContent = this.editActionContent + chunk;
      })
    );

    // Listener for when the stream is complete
    this.unlisteners.push(
      await listen(LLM_EVENTS.COMPLETED, (response) => {
        if (response.event === 'chat-completed') {
          this.isLoading = false;
        }
      })
    );

    // Listener for when the edit action is complete
    this.unlisteners.push(
      await listen(LLM_EVENTS.EDIT_ACTION_COMPLETED, (response) => {
        if (response.event === 'chat-edit-action-completed') {
          this.editActionInProgress = false;
        }
      })
    );

    // Listener for when the RAG chat is complete
    this.unlisteners.push(
      await listen(LLM_EVENTS.RAG_CHAT_COMPLETED, (response) => {
        if (response.event === 'rag-chat-completed') {
          this.isRAGLoading = false;
        }
      })
    );

    // Listener for errors
    this.unlisteners.push(
      await listen(LLM_EVENTS.Error, (response) => {
        console.error('LLM Error:', response.payload.message);
        this.error = `An error occurred: ${response.payload.message}`;
        this.isLoading = false;
        this.editActionInProgress = false;
        this.isRAGLoading = false;
      })
    );

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
   * Sends a RAG (Retrieval-Augmented Generation) message to the Rust backend.
   * @param {string} prompt The user's message.
   */
  async sendRagMessage(prompt) {
    if (this.isRAGLoading || !prompt.trim()) {
      return;
    }
    this.isRAGLoading = true;
    this.error = null;
    this.ragMessages.push({ role: 'user', content: prompt });
    this.ragMessages.push({ role: 'assistant', content: '' });

    try {
      /** @type {any} response */
      const response = await invoke('search_documents', {
        query: prompt,
      });

      const lastMessage = this.ragMessages[this.ragMessages.length - 1];
      if (lastMessage && lastMessage.role === 'assistant') {
        lastMessage.references = response.data;
      }

      return response;
    } catch (e) {
      console.error(e);
      this.error = `An error occurred: ${e}`;
      this.isRAGLoading = false;
      this.ragMessages.pop();
      this.ragMessages.pop(); // Also remove user message on error
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

  newRagSession() {
    this.ragMessages = [];
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
    this.unlisteners.forEach((unlisten) => unlisten());
    this.unlisteners = [];
  }
}

export const llmManager = new LlmManager();
