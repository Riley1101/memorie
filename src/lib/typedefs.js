/**
 * @typedef {Object} Node
 * @property {string} content - The text content of this history state.
 * @property {number | null} parent - The index in the `nodes` array of the parent node.
 * @property {number[]} children - An array of indices for all child nodes.
 */

/**
 * @typedef {Object} History
 * @property {Node[]} nodes - An array (arena) holding all node objects for this history.
 * @property {number | null} current - The index in the `nodes` array of the current state.
 * @property {number[]} redo_stack - A transient stack of indices for managing linear redo.
 */
