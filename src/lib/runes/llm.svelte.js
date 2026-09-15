import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from '$lib/toast.js';

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

  /** How the backend chose to answer: `{ intent, query, documentTitle }`. */
  ROUTE: 'chat-route',
  /** Source notes for a reply that searched notes: `{ results: [{ title, score }] }`. */
  SOURCES: 'chat-sources',
};

const LLM_INVOKE = {
  CHAT: 'run_chat',
  CANCEL_CHAT: 'cancel_chat',
};

const CANCELLED_SUFFIX = '\n\nCancelled by user.';

/** Prior messages sent with each request; the backend trims them further. */
const HISTORY_MESSAGES = 6;

/**
 * @typedef {'user' | 'assistant'} MessageRole
 */

/**
 * @typedef {object} MessageRoute
 * @property {'search_notes' | 'current_document' | 'general'} intent
 * @property {string} query - What was searched for, when intent is `search_notes`.
 * @property {string | null} documentTitle - The open document, if any.
 */

/**
 * @typedef {object} Message
 * @property {MessageRole} role - The role of the message sender.
 * @property {string} content - The text content of the message.
 * @property {MessageRoute} [route] - How the assistant decided to answer.
 * @property {{ title: string, score: number }[]} [references] - Notes a search drew on.
 */

/**
 * @typedef {{ title: string, content: string }} OpenDocument
 */

/**
 * Manages the state and communication for an LLM chat interface in a Tauri app.
 *
 * There is a single chat surface: every message goes through `sendMessage`, and the
 * backend decides whether it's a notes search, a question about the open document, or
 * plain chat, and answers accordingly.
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

  /** @type number - model loading progress **/
  loadingProgress = $state(0);

  /** @type string | null - model loading status **/
  loadingStatus = $state(null);

  /** @type boolean - indicates if load_models command is running **/
  isLoadModelsInProgress = $state(false);

  /** @type Message[] - array of messages **/
  messages = $state([]);

  /** @type boolean - loading state **/
  isLoading = $state(false);

  /** @type string | null - error message **/
  error = $state(null);

  /** @type {Function[]} - array of unlisten functions **/
  unlisteners = $state([]);

  /** @type {Array<{name: string, downloaded: boolean}>} - model status list **/
  modelStatuses = $state([]);

  /** @type {Array<{id: string, name: string, model_type: string, downloaded: boolean}>} - supported models with types **/
  supportedModels = $state([]);

  /** @type {string | null} - id of model currently being downloaded **/
  downloadingModelId = $state(null);

  /** @type {{ modelName: string, at: number } | null} - set when a model download completes (shown in command bar, cleared after 5s) **/
  lastDownloadSuccess = $state(null);

  /** @type {ReturnType<typeof setTimeout> | null} - timeout to clear lastDownloadSuccess **/
  _downloadSuccessClearTimeout = null;

  /**
   * Checks local models and, the first time it's called, subscribes to the backend's
   * streaming events. Safe to call again (e.g. when switching providers): listeners are
   * only registered once, so tokens aren't appended twice.
   */
  async setupModels() {
    this.modelsLoaded = false;

    if (this.unlisteners.length === 0) {
      await this._registerListeners();
    }

    // Initial check of what we have on disk
    await this.checkModels();

    // Auto-load if models are already downloaded
    const anyDownloaded = this.modelStatuses.some(m => m.downloaded);
    if (anyDownloaded) {
      this.loadModels();
    }
  }

  /** @returns {Message | undefined} the in-flight assistant reply, if the last message is one */
  _lastAssistantMessage() {
    const lastMessage = this.messages[this.messages.length - 1];
    return lastMessage?.role === 'assistant' ? lastMessage : undefined;
  }

  async _registerListeners() {
    this.unlisteners.push(
      await listen('model-progress', (event) => {
        this.loadingStatus = event.payload.status;
        this.loadingProgress = event.payload.progress;
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.CHAT_INIT, (response) => {
        if (response.event === 'chat-in-progress') {
          this.isLoading = true;
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
        const lastMessage = this._lastAssistantMessage();
        if (lastMessage) {
          lastMessage.content += /** @type {string} */ (event.payload.response);
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.ROUTE, (event) => {
        const lastMessage = this._lastAssistantMessage();
        if (lastMessage) {
          lastMessage.route = event.payload;
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.SOURCES, (event) => {
        const lastMessage = this._lastAssistantMessage();
        if (lastMessage) {
          lastMessage.references = event.payload.results;
        }
      })
    );

    this.unlisteners.push(
      await listen(LLM_EVENTS.CHAT_IN_PROGRESS, (event) => {
        const lastMessage = this._lastAssistantMessage();
        if (lastMessage) {
          lastMessage.content += /** @type {string} */ (event.payload.content);
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

    // Listener for errors
    this.unlisteners.push(
      await listen(LLM_EVENTS.Error, (response) => {
        console.error('LLM Error:', response.payload.message);
        this.error = `An error occurred: ${response.payload.message}`;
        this.isLoading = false;
        this.editActionInProgress = false;
        this.workerId = null;

        // Clear any dangling "Thinking..." bubble left by the failed request so the UI
        // doesn't get stuck showing an infinite spinner.
        const lastMessage = this._lastAssistantMessage();
        if (lastMessage && lastMessage.content === '') {
          this.messages.pop();
        }
        if (this.editActionContent === '') {
          this.editActionContent = null;
        }

        toast.error('AI request failed', response.payload.message);
      })
    );
  }

  /**
   * Checks the status of AI models on the local machine.
   * Updates modelStatuses with the results.
   */
  async checkModels() {
    try {
      this.modelStatuses = await invoke('check_models');
    } catch (e) {
      console.error('Failed to check models:', e);
    }
  }

  /**
   * Fetches the list of supported Kalosm models with types and download status.
   */
  async fetchSupportedModels() {
    try {
      this.supportedModels = await invoke('get_supported_models');
    } catch (e) {
      console.error('Failed to fetch supported models:', e);
    }
  }

  /**
   * Starts a model download in the background. Progress is shown in the command bar.
   * On success, shows an alert in the command bar and refreshes model lists.
   * @param {string} modelId - e.g. 'qwen_2_5_1_5b_instruct'
   */
  async downloadModel(modelId) {
    if (this.downloadingModelId) return;
    this.downloadingModelId = modelId;
    this.error = null;
    this.lastDownloadSuccess = null;
    if (this._downloadSuccessClearTimeout) {
      clearTimeout(this._downloadSuccessClearTimeout);
      this._downloadSuccessClearTimeout = null;
    }
    try {
      await invoke('download_model', { modelId });
      await this.fetchSupportedModels();
      await this.checkModels();
      const modelName = this.supportedModels.find((m) => m.id === modelId)?.name ?? modelId;
      this.lastDownloadSuccess = { modelName, at: Date.now() };
      toast.success('Model downloaded', modelName);
      this._downloadSuccessClearTimeout = setTimeout(() => {
        this.lastDownloadSuccess = null;
        this._downloadSuccessClearTimeout = null;
      }, 5000);
    } catch (e) {
      const msg = e?.toString?.() ?? String(e);
      console.error('Download failed:', msg);
      this.error = `Download failed: ${msg}`;
      toast.error('Model download failed', msg);
    } finally {
      this.downloadingModelId = null;
      this.loadingProgress = 0;
      this.loadingStatus = null;
    }
  }

  async loadModels() {
    if (this.isLoadModelsInProgress) return;

    this.isLoadModelsInProgress = true;
    this.modelsLoaded = false;

    try {
      await invoke('load_models');
      this.modelsLoaded = true;
      await this.checkModels(); // Refresh status after load/download
    } catch (e) {
      console.error('Failed to load models:', e);
      this.error = `Model initialization failed: ${e}`;
      toast.error('Could not load AI model', e);
    } finally {
      this.isLoadModelsInProgress = false;
      this.loadingStatus = null;
      this.loadingProgress = 0;
    }
  }

  /**
   * Sends a message along with the last few turns and the open document. The backend
   * decides whether to search notes, answer from the document, or just chat.
   * Prefix with `/search`, `/doc` or `/chat` to choose explicitly.
   * @param {string} prompt The user's message.
   * @param {OpenDocument | null} [document] The open document, if any.
   * @returns {Promise<void>}
   */
  async sendMessage(prompt, document = null) {
    if (this.isLoading || !prompt.trim()) {
      return;
    }

    const history = this.messages
      .filter((m) => m.content.trim() && !m.content.endsWith(CANCELLED_SUFFIX))
      .slice(-HISTORY_MESSAGES)
      .map(({ role, content }) => ({ role, content }));

    this.isLoading = true;
    this.error = null;

    this.messages.push({ role: 'user', content: prompt });
    this.messages.push({ role: 'assistant', content: '' });

    try {
      /** @type {string} processId */
      this.workerId = await invoke(LLM_INVOKE.CHAT, {
        message: prompt,
        mode: 'Normal',
        context: {
          history,
          documentTitle: document?.title ?? null,
          documentContent: document?.content ?? null,
        },
      });
    } catch (e) {
      console.error(e);
      this.error = `An error occurred: ${e}`;
      this.isLoading = false;
      this.messages.splice(-2, 2);
      toast.error('Could not send message', e);
    }
  }

  /**
   * Regenerates the assistant reply for the exchange ending at `assistantIndex`
   * by re-sending the user prompt just before it. Only the latest exchange can
   * be retried.
   * @param {number} assistantIndex
   * @param {OpenDocument | null} [document]
   * @returns {Promise<void>}
   */
  async retryMessage(assistantIndex, document = null) {
    if (this.isLoading) return;
    const isLast = assistantIndex === this.messages.length - 1;
    const user = this.messages[assistantIndex - 1];
    if (!isLast || !user || user.role !== 'user') return;

    const prompt = user.content;
    this.messages.splice(assistantIndex - 1, 2);
    await this.sendMessage(prompt, document);
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
        this.editActionInProgress = false;
        this.workerId = null;

        const lastMessage = this._lastAssistantMessage();
        if (lastMessage) {
          lastMessage.content += CANCELLED_SUFFIX;
        }

        if (this.editActionContent !== null) {
          this.editActionContent += CANCELLED_SUFFIX;
          this.newEditActionSession();
        }
      }
    }
  }

  /**
   * @public
   * Starts a new conversation. History lives only in `messages`, so clearing it is enough.
   */
  newSession() {
    this.messages = [];
    this.error = null;
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
