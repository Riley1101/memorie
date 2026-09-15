/**
 * Subsequence fuzzy match with a lightweight score. Returns null when the
 * query's characters don't all appear in order within the target.
 * @param {string} query
 * @param {string} target
 * @returns {number | null}
 */
export function fuzzyScore(query, target) {
  if (!query) return 0;

  const q = query.toLowerCase();
  const t = target.toLowerCase();

  let score = 0;
  let tIndex = 0;
  let consecutive = 0;

  for (let qIndex = 0; qIndex < q.length; qIndex++) {
    const char = q[qIndex];
    const foundAt = t.indexOf(char, tIndex);
    if (foundAt === -1) return null;

    consecutive = foundAt === tIndex ? consecutive + 1 : 0;
    score += foundAt === 0 ? 3 : consecutive > 0 ? 2 : 1;
    tIndex = foundAt + 1;
  }

  return score;
}
