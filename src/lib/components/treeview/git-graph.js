/** Trunk reads as the clay accent; every other branch is teal — the app's two
 *  brand colors, so history stays legible without a rainbow of lane colors. */
export const LANE_COLORS = ['var(--color-primary)', 'var(--color-teal)'];

/**
 * @typedef {Object} HistoryNode
 * @property {string} [preview] - Short excerpt of the version's text.
 * @property {number | null} [createdAt] - Milliseconds since the epoch.
 * @property {number | null} parent
 * @property {number[]} children
 */

/**
 * @typedef {Object} GraphRow
 * @property {number} id
 * @property {number} lane
 * @property {string} color
 * @property {number | null} parentId
 * @property {number | null} parentLane
 * @property {boolean} isBranchStart - true when this is the oldest commit of
 *   a non-trunk lane, i.e. where it should curve back into its parent lane.
 * @property {boolean} hasIncoming - true when a newer commit exists above in
 *   the same lane (so a line should be drawn into the top of this row).
 * @property {boolean} isRoot
 * @property {boolean} isCurrent - true for the version currently checked out.
 * @property {string} preview
 * @property {number | null} createdAt
 * @property {{ lane: number, color: string }[]} passthrough - other lanes
 *   that are "in flight" through this row and need a plain vertical segment.
 */

/**
 * Lays out a version-history tree (one parent per node, no merges) as a
 * git-graph style structure: each branch point opens a new lane/color, lanes
 * run parallel to the trunk, and a lane curves back into its parent lane at
 * its oldest commit. Rows are ordered newest-first (by id, descending),
 * mirroring how `git log --graph` renders.
 *
 * @param {HistoryNode[]} nodes
 * @param {number | null} current
 * @returns {{ rows: GraphRow[], laneCount: number }}
 */
export function buildGitGraph(nodes, current) {
  if (!nodes || nodes.length === 0) return { rows: [], laneCount: 0 };

  const rootId = nodes.findIndex((n) => n.parent === null);
  if (rootId === -1) return { rows: [], laneCount: 0 };

  const laneOf = new Array(nodes.length).fill(-1);
  let laneCounter = 1; // lane 0 is reserved for the trunk

  /** @param {number} id @param {number} lane */
  function assign(id, lane) {
    laneOf[id] = lane;
    const kids = nodes[id].children;
    if (!kids || kids.length === 0) return;
    assign(kids[0], lane);
    for (let i = 1; i < kids.length; i++) {
      assign(kids[i], laneCounter++);
    }
  }
  assign(rootId, 0);

  const order = laneOf
    .map((lane, id) => (lane === -1 ? -1 : id))
    .filter((id) => id !== -1)
    .sort((a, b) => b - a);

  const laneRange = new Map();
  for (const id of order) {
    const lane = laneOf[id];
    const range = laneRange.get(lane);
    if (!range) laneRange.set(lane, { min: id, max: id });
    else {
      if (id < range.min) range.min = id;
      if (id > range.max) range.max = id;
    }
  }

  const maxLane = laneCounter - 1;

  const rows = order.map((id) => {
    const node = nodes[id];
    const lane = laneOf[id];
    const range = laneRange.get(lane);
    const parentId = node.parent;
    const parentLane = parentId !== null ? laneOf[parentId] : null;

    /** @type {{ lane: number, color: string }[]} */
    const passthrough = [];
    for (let l = 0; l <= maxLane; l++) {
      if (l === lane) continue;
      const r = laneRange.get(l);
      if (r && r.min <= id && id <= r.max) {
        passthrough.push({ lane: l, color: LANE_COLORS[l % LANE_COLORS.length] });
      }
    }

    return {
      id,
      lane,
      color: LANE_COLORS[lane % LANE_COLORS.length],
      parentId,
      parentLane,
      isBranchStart: parentId !== null && id === range.min && parentLane !== lane,
      hasIncoming: id !== range.max,
      isRoot: parentId === null,
      isCurrent: id === current,
      preview: node.preview ?? '',
      createdAt: node.createdAt ?? null,
      passthrough,
    };
  });

  return { rows, laneCount: maxLane + 1 };
}
