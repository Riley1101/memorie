/**
 * @typedef MemoryContext {
 *     content: string;
 *     fileName: string;
 * }
 */

/**
 * Manages memory-related functionalities for runes.
 */
class MemoryManager {

    /** @type {{content: string, fileName: string}} */
    context = $state ({content: "", fileName: ""});
    setContext(content, fileName) {
        this.context = {
            content,
            fileName,
        }
    }
}

export let memoryManager = new MemoryManager();