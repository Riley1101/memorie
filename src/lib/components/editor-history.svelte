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
  import TreeNode from './treeview/treenode.svelte';

  /** @type {{ fileName:string , history: History}} */
  let { fileName, history } = $props();

  let rootNodeData = $derived.by(() => {
    const nodes = history?.nodes || [];
    const rootIndex = nodes.findIndex((node) => node.parent === null);

    if (rootIndex === -1) {
      return { node: null, id: -1 };
    }

    return { node: nodes[rootIndex], id: rootIndex };
  });
</script>

<div class="p-5 pb-[50vh]">
  <!-- Cleaner conditional rendering with better null handling -->
  <div class="pl-2">
    {#if rootNodeData.node}
      <TreeNode
        id={rootNodeData.id}
        node={rootNodeData.node}
        allNodes={history.nodes}
        current={history.current}
        {fileName}
      />
    {:else}
      <p class="text-sm text-muted-foreground">No history available</p>
    {/if}
  </div>
</div>
