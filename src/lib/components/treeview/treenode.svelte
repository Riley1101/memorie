<script>
  import Tree_node from './treenode.svelte';
  import { cn } from '$lib/utils';
  import { invalidateAll } from '$app/navigation';
  import Button from '$lib/components/ui/button/button.svelte';
  import { invoke } from '@tauri-apps/api/core';

  /**
   * @typedef {Object} Node
   * @property {number} [id] - The unique identifier of this history state.
   * @property {string} content - The text content of this history state.
   * @property {number | null} parent - The index in the `nodes` array of the parent node.
   * @property {number[]} children - An array of indices for all child nodes.
   */

  /**
   * @type {{ node: Node, id: number, allNodes: Node[], current: number | null, fileName: string }}
   */
  let { node, id, allNodes, current, fileName } = $props();

  let isNavigating = $state(false);

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
   * Handles navigation to a specific version when this node is clicked.
   * @param {number} nodeId - The ID of the node to navigate to.
   */
  async function handleUndotoVersion(nodeId) {
    if (isNavigating || isCurrent) return;

    isNavigating = true;

    try {
      await invoke('goto_file_version', {
        name: fileName,
        nodeId: nodeId,
      });
      await invalidateAll();
    } catch (error) {
      console.error(`Failed to navigate to version ${nodeId}:`, error);
    } finally {
      isNavigating = false;
    }
  }

  let hasMultipleChildren = $derived(children.length > 1);
  let hasChildren = $derived(children.length > 0);
</script>

<div class="w-full text-xs">
  {#if hasChildren}
    <!-- Improved button with better accessibility and states -->
    <Button
      onclick={() => handleUndotoVersion(id)}
      variant="ghost"
      size="sm"
      disabled={isNavigating || isCurrent}
      class={cn(
        'w-full justify-start border-l border-dashed transition-colors',
        isCurrent && 'bg-accent text-accent-foreground font-medium',
        !isCurrent && 'text-muted-foreground hover:text-foreground',
        isNavigating && 'opacity-50 cursor-wait'
      )}
      aria-label={`Navigate to ${nodeName}`}
      aria-current={isCurrent ? 'location' : undefined}
    >
      {nodeName}
    </Button>

    <!-- Better tree structure with consistent indentation -->
    <div class={cn('pt-1', hasMultipleChildren && 'pl-5 border-l border-dashed')}>
      {#each children as child (child.id)}
        <Tree_node id={child.id} node={child} {allNodes} {current} {fileName} />
      {/each}
    </div>
  {:else}
    <!-- Leaf node with better spacing -->
    <Button
      onclick={() => handleUndotoVersion(id)}
      size="sm"
      variant="ghost"
      disabled={isNavigating || isCurrent}
      class={cn(
        'border-l border-dashed w-full justify-start mb-1 transition-colors',
        isCurrent && 'bg-accent text-accent-foreground font-medium',
        !isCurrent && 'text-muted-foreground hover:text-foreground',
        isNavigating && 'opacity-50 cursor-wait'
      )}
      aria-label={`Navigate to ${nodeName}`}
      aria-current={isCurrent ? 'location' : undefined}
    >
      {nodeName}
    </Button>
  {/if}
</div>
