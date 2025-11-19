import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/**
 * LLM events
 * @type {{CHAT_IN_PROGRESS: string, COMPLETED: string, Error: string}}
 *
 * This enum map directly to ChatEvent enum in Rust backend
 * @see ./src-tauri/workers.rs
 */
const LLM_EVENTS = {
  CHAT_INIT: 'chat-init',
  CHAT_IN_PROGRESS: 'chat-in-progress',
  AUTOCOMPOLETE: 'chat-autocomplete',
  COMPLETED: 'chat-completed',
  Error: 'chat-error',
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
  async setupListeners() {
    console.info('Setting up LLM listeners');
    // Listener for incoming text chunks


    await listen(LLM_EVENTS.CHAT_INIT,(response)=>{
      if (response.event === "chat-in-progress"){
        this.isLoading = true;
      }
    })

    await listen(LLM_EVENTS.AUTOCOMPOLETE, (event) => {
      console.log(event)
      const chunk = /** @type {string} */ (event.payload.response);
      if (this.messages.length > 0) {
        console.log("Received chunk:", chunk);
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

    // Listener for when the stream is complete
    this.unlistenDone = await listen(LLM_EVENTS.COMPLETED, (response) => {
      if(response.event === "chat-completed"){
        this.isLoading = false;
      }
    });
  }

  /**
   * Sends a user's prompt to the Rust backend to start the LLM stream.
   * @param {string} prompt The user's message.
   * @param {string} [mode] The mode of the chat (default is "Normal").
   * @returns {Promise<void>}
   */
  async sendMessage(prompt, mode = "Normal") {
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

  async cancelMessage() {
    if (this.workerId) {
      let status = await invoke(LLM_INVOKE.CANCEL_CHAT, { jobId: this.workerId });
      if (status) {
        this.isLoading = false;
        this.workerId = null;
        this.messages.pop();
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
