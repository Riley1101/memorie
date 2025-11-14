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

  import HistoryIcon from '@lucide/svelte/icons/history';
  import TreeNode from './treeview/treenode.svelte';

  /** @type {{ fileName:string , history: History}} */
  let { fileName, history } = $props();

  let allNodes = $derived(history?.nodes || []);

  let rootId = $state(-1);

  let rootNode = allNodes.find((node, index) => {
    if (node.parent === null) {
      rootId = index;
      return true;
    }
    return false;
  });
</script>

<div class="p-5 pb-[50vh]">
  <div class="flex items-center gap-2 mb-4 text-muted-foreground">
    <HistoryIcon class="size-5 " />
    <h2 class="mt-1">Undotree</h2>
  </div>
  <div class="pl-2">
    {#if rootNode}
      <TreeNode id={rootId} node={rootNode} {allNodes} current={history?.current} {fileName} />
    {/if}
  </div>
</div>
