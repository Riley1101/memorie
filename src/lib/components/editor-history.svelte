<script>
  /**
   * @typedef {Object} Node
   * @property {string} [preview] - A short excerpt of this history state.
   * @property {number | null} [createdAt] - When this state was saved, in ms since the epoch.
   * @property {number | null} parent - The index in the `nodes` array of the parent node.
   * @property {number[]} children - An array of indices for all child nodes.
   */

  /**
   * @typedef {Object} History
   * @property {Node[]} nodes - An array (arena) holding all node objects for this history.
   * @property {number | null} current - The index in the `nodes` array of the current state.
   * @property {number[]} redo_stack - A transient stack of indices for managing linear redo.
   */
  import GitGraph from './treeview/GitGraph.svelte';

  /** @type {{ fileName:string , history: History}} */
  let { fileName, history } = $props();
</script>

<div class="p-3 pb-[50vh]">
  <GitGraph {fileName} allNodes={history?.nodes ?? []} current={history?.current ?? null} />
</div>
