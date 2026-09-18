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

  // Rows are two lines tall — a version label and an excerpt — so the rail is
  // laid out against the label line rather than the middle of the row.
  const ROW_HEIGHT = 44;
  const LANE_WIDTH = 14;
  const DOT_RADIUS = 4;
  const DOT_OFFSET = 9;

  let graph = $derived.by(() => buildGitGraph(allNodes, current));
  let lanesWidth = $derived(Math.max(graph.laneCount, 1) * LANE_WIDTH + 6);
  let height = $derived(graph.rows.length * ROW_HEIGHT);

  let navigatingId = $state(/** @type {number | null} */ (null));

  /** @param {number} lane */
  function laneX(lane) {
    return lane * LANE_WIDTH + LANE_WIDTH / 2;
  }

  /**
   * Time of day for versions saved today, a short date for older ones — the
   * label has room for one or the other, not both.
   * @param {number | null | undefined} ms
   */
  function formatStamp(ms) {
    if (!ms) return null;
    const date = new Date(ms);
    if (Number.isNaN(date.getTime())) return null;
    const today = new Date();
    const sameDay =
      date.getFullYear() === today.getFullYear() &&
      date.getMonth() === today.getMonth() &&
      date.getDate() === today.getDate();
    return sameDay
      ? date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
      : date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
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
    <div class="relative" style="height: {height}px;">
      <svg
        width={lanesWidth}
        {height}
        class="pointer-events-none absolute left-0 top-0"
        viewBox="0 0 {lanesWidth} {height}"
        aria-hidden="true"
      >
        {#each graph.rows as row, index (row.id)}
          {@const y0 = index * ROW_HEIGHT}
          {@const yDot = y0 + DOT_OFFSET}
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
              stroke-width="1.5"
            />
          {/each}

          <!-- incoming segment from the newer commit above in this lane -->
          {#if row.hasIncoming}
            <line x1={x} y1={y0} x2={x} y2={yDot} stroke={row.color} stroke-width="1.5" />
          {/if}

          <!-- outgoing segment: either curves into the parent lane (branch
               point) or continues straight down in the same lane -->
          {#if row.isBranchStart && row.parentLane !== null}
            {@const px = laneX(row.parentLane)}
            <path
              d="M {x} {yDot} C {x} {yDot + ROW_HEIGHT / 2}, {px} {yDot + ROW_HEIGHT / 3}, {px} {yEnd}"
              fill="none"
              stroke={row.color}
              stroke-width="1.5"
            />
          {:else if !row.isRoot}
            <line x1={x} y1={yDot} x2={x} y2={yEnd} stroke={row.color} stroke-width="1.5" />
          {/if}

          {#if row.isCurrent}
            <circle cx={x} cy={yDot} r={DOT_RADIUS + 5} fill={row.color} opacity="0.25" />
          {/if}
          <circle
            cx={x}
            cy={yDot}
            r={DOT_RADIUS}
            fill={row.isCurrent ? row.color : 'var(--color-background)'}
            stroke={row.color}
            stroke-width="1.5"
          />
        {/each}
      </svg>

      {#each graph.rows as row, index (row.id)}
        {@const y0 = index * ROW_HEIGHT}
        {@const stamp = formatStamp(row.createdAt)}
        <button
          type="button"
          onclick={() => goto(row)}
          disabled={navigatingId !== null || row.isCurrent}
          class={cn(
            'absolute left-0 w-full rounded-md pr-2 text-left transition-colors hover:bg-foreground/5',
            row.isCurrent ? 'cursor-default' : 'cursor-pointer',
            navigatingId === row.id && 'cursor-wait opacity-50'
          )}
          style="top: {y0}px; height: {ROW_HEIGHT}px; padding-left: {lanesWidth + 4}px;"
          aria-label={row.isCurrent ? `v${row.id}, current version` : `Go to version v${row.id}`}
        >
          <p
            class={cn(
              'flex items-center gap-1 font-mono text-[10px] leading-4.5',
              row.isCurrent ? 'text-primary' : 'text-metadata'
            )}
          >
            <span>v{row.id}</span>
            {#if stamp}<span>· {stamp}</span>{/if}
            {#if row.isCurrent}
              <span>· current</span>
            {:else if row.isBranchStart}
              <span class="rounded border border-border px-1 py-px text-[9px] leading-none">
                branch
              </span>
            {/if}
          </p>
          <p
            class={cn(
              'truncate text-[11px] leading-4',
              row.isCurrent ? 'text-foreground' : 'text-muted-foreground'
            )}
          >
            {row.preview || 'Empty'}
          </p>
        </button>
      {/each}
    </div>
  {/if}
</div>
