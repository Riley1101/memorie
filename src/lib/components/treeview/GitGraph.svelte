<script>
  import { cn } from '$lib/utils';
  import { invalidateAll } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { appState } from '$lib/runes/app.svelte.js';
  import { buildGitGraph } from './git-graph.js';

  /**
   * @type {{ fileName: string, allNodes: import('./git-graph.js').HistoryNode[], current: number | null }}
   */
  let { fileName, allNodes, current } = $props();

  const ROW_HEIGHT = 29;
  const LANE_WIDTH = 16;
  const DOT_RADIUS = 4;
  const HIT_HEIGHT = ROW_HEIGHT;
  const LABEL_WIDTH = 32;

  let graph = $derived.by(() => buildGitGraph(allNodes, current));
  let lanesWidth = $derived(Math.max(graph.laneCount, 1) * LANE_WIDTH);
  let width = $derived(lanesWidth + LABEL_WIDTH);
  let height = $derived(graph.rows.length * ROW_HEIGHT);

  let navigatingId = $state(/** @type {number | null} */ (null));

  /** @param {number} lane */
  function laneX(lane) {
    return lane * LANE_WIDTH + LANE_WIDTH / 2;
  }

  /** @param {import('./git-graph.js').GraphRow} row */
  async function goto(row) {
    if (navigatingId !== null || row.isCurrent) return;

    navigatingId = row.id;
    try {
      await invoke('goto_file_version', { name: fileName, nodeId: row.id });
      await invalidateAll();
      appState.incrementEditorVersion();
    } catch (error) {
      console.error(`Failed to navigate to version ${row.id}:`, error);
    } finally {
      navigatingId = null;
    }
  }
</script>

<div class="w-full text-xs">
  {#if graph.rows.length === 0}
    <p class="p-3 text-muted-foreground">No history available</p>
  {:else}
    <div class="overflow-x-auto overflow-y-hidden">
      <div class="relative" style="height: {height}px; width: {width}px; min-width: 100%;">
        <svg
          {width}
          {height}
          class="absolute left-0 top-0"
          viewBox="0 0 {width} {height}"
          aria-hidden="true"
        >
          {#each graph.rows as row, index (row.id)}
            {@const y0 = index * ROW_HEIGHT}
            {@const yMid = y0 + ROW_HEIGHT / 2}
            {@const yEnd = y0 + ROW_HEIGHT}
            {@const x = laneX(row.lane)}

            <!-- plain passthrough lines for lanes just riding through this row -->
            {#each row.passthrough as p (p.lane)}
              <line
                x1={laneX(p.lane)}
                y1={y0}
                x2={laneX(p.lane)}
                y2={yEnd}
                stroke={p.color}
                stroke-width="2"
              />
            {/each}

            <!-- incoming segment from the newer commit above in this lane -->
            {#if row.hasIncoming}
              <line x1={x} y1={y0} x2={x} y2={yMid} stroke={row.color} stroke-width="2" />
            {/if}

            <!-- outgoing segment: either curves into the parent lane (branch
                 point) or continues straight down in the same lane -->
            {#if row.isBranchStart && row.parentLane !== null}
              {@const px = laneX(row.parentLane)}
              <path
                d="M {x} {yMid} C {x} {yMid + ROW_HEIGHT / 2}, {px} {yMid}, {px} {yEnd}"
                fill="none"
                stroke={row.color}
                stroke-width="2"
              />
            {:else if !row.isRoot}
              <line x1={x} y1={yMid} x2={x} y2={yEnd} stroke={row.color} stroke-width="2" />
            {/if}

            {#if row.isCurrent}
              <circle cx={x} cy={yMid} r={DOT_RADIUS + 5} fill={row.color} opacity="0.25" />
            {/if}
            <circle
              cx={x}
              cy={yMid}
              r={DOT_RADIUS}
              fill={row.isCurrent ? row.color : 'var(--color-background)'}
              stroke={row.color}
              stroke-width="2"
            />

            <text
              x={lanesWidth + 4}
              y={yMid}
              dominant-baseline="middle"
              font-size="10"
              fill={row.isCurrent ? row.color : 'var(--color-muted-foreground)'}
            >v{row.id}</text>
          {/each}
        </svg>

        {#each graph.rows as row, index (row.id)}
          {@const y0 = index * ROW_HEIGHT}
          <button
            type="button"
            onclick={() => goto(row)}
            disabled={navigatingId !== null || row.isCurrent}
            class={cn(
              'absolute left-0 w-full transition-colors hover:bg-foreground/10',
              row.isCurrent ? 'cursor-default' : 'cursor-pointer',
              navigatingId === row.id && 'opacity-50 cursor-wait'
            )}
            style="top: {y0}px; height: {HIT_HEIGHT}px;"
            aria-label={row.isRoot ? 'Root' : `Navigate to v${row.id}`}
          ></button>
        {/each}
      </div>
    </div>
  {/if}
</div>
