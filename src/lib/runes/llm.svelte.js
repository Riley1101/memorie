import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/**
 * LLM events
 * @type {{CHAT_IN_PROGRESS: string, COMPLETED: string, Error: string}}
 */
const LLM_EVENTS = {
    CHAT_IN_PROGRESS: 'chat-in-progress',
    COMPLETED: 'chat-completed',
    Error:"chat-error"
}

const LLM_INVOKE = {
    CHAT: 'run_chat'
}

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
    /** @type {import('svelte/reactivity').Rune<Message[]>} */
    messages = $state([]);

    /** @type {import('svelte/reactivity').Rune<boolean>} */
    isLoading = $state(false);

    /** @type {import('svelte/reactivity').Rune<string | null>} */
    error = $state(null);

    /** @private */
    unlistenChunk = null;
    /** @private */
    unlistenDone = null;


    /**
     * Sets up Tauri event listeners to receive streaming data from the Rust backend.
     */
    async setupListeners() {
        console.info("Setting up LLM listeners");
        // Listener for incoming text chunks
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
        this.unlistenDone = await listen(LLM_EVENTS.COMPLETED, () => {
            this.isLoading = false;
        });
    }

    /**
     * Sends a user's prompt to the Rust backend to start the LLM stream.
     * @param {string} prompt The user's message.
     * @returns {Promise<void>}
     */
    async sendMessage(prompt) {
        if (this.isLoading || !prompt.trim()) {
            return; // Prevent sending empty messages or multiple requests
        }

        this.isLoading = true;
        this.error = null;

        this.messages.push({ role: 'user', content: prompt });
        this.messages.push({ role: 'assistant', content: '' });

        try {
            await invoke(LLM_INVOKE.CHAT, { message: prompt });
        } catch (e) {
            console.error(e);
            this.error = `An error occurred: ${e}`;
            this.isLoading = false;
            this.messages.pop();
        }
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