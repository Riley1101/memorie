import { countWords } from '$lib/runes/writing.svelte.js';

/**
 * @file Per-block caches for work that otherwise walks the whole document.
 *
 * ProseMirror keeps a top-level block as the same object for as long as it is
 * unchanged, so results keyed on the node (in a WeakMap) only need computing
 * for the few blocks an edit touched. In a 500k-word manuscript that turns a
 * whole-book pass into a pass over one or two paragraphs.
 */

/** @typedef {import('@milkdown/kit/prose/model').Node} PMNode */

/** @type {WeakMap<PMNode, { words: number, chars: number }>} */
const countCache = new WeakMap();

/**
 * Word and character counts for the document, the same numbers
 * `countWords(doc.textBetween(0, size, '\n', ' '))` gives, summed per block.
 * Blocks are joined by a newline there, so no word spans two of them.
 * @param {PMNode} doc
 */
export function documentCounts(doc) {
  let words = 0;
  let chars = 0;
  doc.forEach((block) => {
    let hit = countCache.get(block);
    if (!hit) {
      const text = block.textBetween(0, block.content.size, '\n', ' ');
      hit = { words: countWords(text), chars: text.replace(/\s/g, '').length };
      countCache.set(block, hit);
    }
    words += hit.words;
    chars += hit.chars;
  });
  return { words, chars };
}

/** @type {WeakMap<PMNode, { nodes: PMNode[], md: string }>} */
const markdownCache = new WeakMap();

/**
 * Top-level nodes whose Markdown can depend on their neighbours: two lists in
 * a row need a separator the serializer only adds when it sees both, and raw
 * HTML can swallow what follows it. These are serialized together with the
 * blocks either side of them.
 * @param {PMNode} node
 */
function joinSensitive(node) {
  const name = node.type.name;
  return name === 'bullet_list' || name === 'ordered_list' || name === 'html';
}

/**
 * Splits the document into runs: one block, or blocks tied together by a
 * list or HTML block between them.
 * @param {PMNode} doc
 * @returns {PMNode[][]}
 */
function runsOf(doc) {
  /** @type {PMNode[][]} */
  const runs = [];
  /** @type {PMNode | null} */
  let prev = null;
  doc.forEach((block) => {
    const last = runs.at(-1);
    if (last && prev && (joinSensitive(prev) || joinSensitive(block))) last.push(block);
    else runs.push([block]);
    prev = block;
  });
  return runs;
}

/** @param {PMNode[]} nodes */
function cached(nodes) {
  const hit = markdownCache.get(nodes[0]);
  if (hit && hit.nodes.length === nodes.length && hit.nodes.every((n, i) => n === nodes[i])) {
    return hit.md;
  }
  return undefined;
}

/**
 * @param {PMNode} doc
 * @param {PMNode[]} nodes
 * @param {(doc: PMNode) => string} serializeAll
 */
function serializeRun(doc, nodes, serializeAll) {
  const md = serializeAll(doc.type.create(doc.attrs, nodes)).replace(/\n+$/, '');
  markdownCache.set(nodes[0], { nodes, md });
  return md;
}

/**
 * Serializes `doc` to Markdown a run of top-level blocks at a time, reusing
 * the Markdown of runs that haven't changed. Gives the same output as
 * `serializeAll(doc)`.
 * @param {PMNode} doc
 * @param {(doc: PMNode) => string} serializeAll - Milkdown's serializer.
 * @returns {string}
 */
export function serializeIncremental(doc, serializeAll) {
  const parts = runsOf(doc).map((nodes) => cached(nodes) ?? serializeRun(doc, nodes, serializeAll));
  return parts.join('\n\n') + '\n';
}

/**
 * Fills the Markdown cache for `doc` for up to `budgetMs`, so the first save
 * after opening a long document is as quick as the later ones.
 * @param {PMNode} doc
 * @param {(doc: PMNode) => string} serializeAll
 * @param {number} budgetMs
 * @returns {boolean} Whether every run is now cached.
 */
export function warmMarkdownCache(doc, serializeAll, budgetMs) {
  const until = performance.now() + budgetMs;
  for (const nodes of runsOf(doc)) {
    if (cached(nodes) !== undefined) continue;
    if (performance.now() > until) return false;
    serializeRun(doc, nodes, serializeAll);
  }
  return true;
}
