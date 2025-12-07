<script>
  import { cn } from '@/utils';
  import { invalidateAll } from '$app/navigation';
  import TreeNode from './treenode.svelte';
  import Button from '../ui/button/button.svelte';
  import { invoke } from '@tauri-apps/api/core';

  /**
   * @typedef {Object} Node
   * @property {number} [id] - The unique identifier of this history state.
   * @property {string} content - The text content of this history state.
   * @property {number | null} parent - The index in the `nodes` array of the parent node.
   * @property {number[]} children - An array of indices for all child nodes.
   */

  /**
   * @type {{ node: Node , id: number , allNodes: Node[] , current: number | null, fileName: string}}
   */
  let { node, id, allNodes, current, fileName } = $props();

  /**
   * A derived array of the full child node objects.
   * @type {Array<Node & {id: number}>}
   */
  let children = $derived(
    node.children.map((childId) => ({
      ...allNodes[childId],
      id: childId,
    }))
  );

  /**
   * A derived boolean that is true if this node is the currently selected node.
   * @type {boolean}
   */
  let isCurrent = $derived(id === current);

  let nodeName = $derived(node.id ? `Version ${node.id}` : 'Root');

  /**
   * Handles the undoto action when this node is clicked.
   * @param nodeId {number} - The ID of the node to undoto to.
   */
  async function handleUndotoVersion(nodeId) {
    await invoke('goto_file_version', {
      name: fileName,
      nodeId: nodeId,
    })
      .then(() => {
        invalidateAll();
      })
      .catch((e) => {
        console.error('Error going to file version:', e);
      });
  }
</script>

<div class="text-muted-foreground w-full text-xs">
  {#if children.length > 0}
    <Button
      onclick={() => handleUndotoVersion(id)}
      variant="ghost"
      size="sm"
      class={cn('w-full justify-start border-l border-dashed', isCurrent && 'text-primary')}
    >
      {nodeName}
    </Button>
    <div class={cn('border-dashed pt-2', children.length > 1 ? 'pl-5 border-l ' : '')}>
      {#each children as child (child.id)}
        <TreeNode id={child.id} node={child} {allNodes} {current} {fileName} />
      {/each}
    </div>
  {:else}
    <Button
      onclick={() => handleUndotoVersion(id)}
      size="sm"
      variant="ghost"
      class={cn('border-l border-dashed w-full justify-start mb-2', isCurrent && 'text-primary')}
    >
      {nodeName}
    </Button>
  {/if}
</div>
