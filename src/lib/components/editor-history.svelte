<script>
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

  import { Button } from '$lib/components/ui/button/index.js';
  import { fileManager } from '$lib/runes/fs.svelte';

  /** @type {{ fileName:string , history: History}} */
  let { fileName, history } = $props();

  function handleUndo() {
    fileManager.undoFile(fileName);
  }

</script>

<div class="w-full p-2">
  <Button onclick={handleUndo} disabled={history.current === null} class="mb-2">
    Undo
  </Button>
  <code>
    <pre>
      {JSON.stringify(history, null, 2)}
    </pre>
  </code>
</div>
