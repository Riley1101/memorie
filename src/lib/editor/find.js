/**
 * @file Find and replace against the Rust engine.
 *
 * The Milkdown plugin this replaces keeps its matches as document positions and
 * maps them through every transaction, rescanning only once typing pauses. None
 * of that is here, for the same reason none of it is in `decorations.js`: a
 * match is derived from the text, so when the text changes the honest answer is
 * to scan it again. The scan is over the blocks the view already holds — no
 * round trip to Rust — and it costs a regex pass over the document, which at
 * 120k characters is well inside a frame.
 *
 * What *is* kept from the plugin is the part that was never about ProseMirror:
 * {@link buildFindRegex} and the `$1` expansion, which are reused from it
 * directly rather than reimplemented.
 */

import { flattenBlocks, textOfRuns } from './decorations.js';

/**
 * Builds the search pattern, or throws on a bad regular expression.
 *
 * Whole word counts any letter or digit as part of a word, like the backend's
 * Unicode `\b`; JavaScript's `\b` only knows ASCII, so "café" would never
 * match. Lives here rather than in the Milkdown plugin — which is where it
 * started — because this module must not load ProseMirror to find a word, and
 * the plugin imports it back from here so there is one implementation.
 * @param {string} query
 * @param {{ caseSensitive: boolean, wholeWord: boolean, regex: boolean }} options
 * @returns {RegExp}
 */
export function buildFindRegex(query, options) {
  let source = options.regex ? query : query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  if (options.wholeWord) source = `(?<![\\p{L}\\p{N}_])(?:${source})(?![\\p{L}\\p{N}_])`;
  return new RegExp(source, options.caseSensitive ? 'gu' : 'giu');
}

/**
 * @typedef {Object} FindMatch
 * @property {number} block
 * @property {number} start - UTF-16 offset in the block.
 * @property {number} end
 * @property {RegExpMatchArray} groups - For `$1` in a replacement.
 */

/**
 * Past this many we stop collecting, so a one-letter query in a novel stays
 * responsive. The same ceiling the Milkdown plugin used.
 */
const MAX_MATCHES = 5000;

/** Stands in for an image or other inline atom, so offsets line up. */
const LEAF = '￼';

/**
 * Every match in the document, in document order.
 *
 * A match that takes in an atom is skipped: replacing it would delete an image,
 * and the writer asked to replace text.
 * @param {import('$lib/rust-editor.js').EditorBlock[]} blocks
 * @param {RegExp} regex - Global.
 * @param {number} [limit]
 * @returns {FindMatch[]}
 */
export function findAll(blocks, regex, limit = MAX_MATCHES) {
  /** @type {FindMatch[]} */
  const matches = [];
  for (const block of flattenBlocks(blocks)) {
    if (matches.length >= limit) break;
    const text = block.runs ? textOfRuns(block.runs) : (block.text ?? '');
    if (!text) continue;
    regex.lastIndex = 0;
    let match;
    while ((match = regex.exec(text)) !== null) {
      if (!match[0]) {
        regex.lastIndex += 1;
        continue;
      }
      if (!match[0].includes(LEAF)) {
        matches.push({
          block: block.id,
          start: match.index,
          end: match.index + match[0].length,
          groups: match,
        });
        if (matches.length >= limit) break;
      }
    }
  }
  return matches;
}

/**
 * The matches as decorations, with the current one marked so it can be told
 * apart from the rest.
 * @param {FindMatch[]} matches
 * @param {number} current - Index into `matches`; -1 for none.
 * @returns {import('./decorations.js').Decoration[]}
 */
export function findDecorations(matches, current) {
  return matches.map((match, index) => ({
    block: match.block,
    start: match.start,
    end: match.end,
    class: index === current ? 'deco-find-current' : 'deco-find',
  }));
}

/**
 * Index of the first match at or after a position, wrapping to the first.
 *
 * "At or after" is in document order, which for the engine means the block's
 * place in the block list and then the offset inside it — so the caller passes
 * the order the blocks are in rather than the engine's ids, which say nothing
 * about position.
 * @param {FindMatch[]} matches
 * @param {number[]} order - Block ids in document order.
 * @param {{ block: number, offset: number }} from
 * @returns {number}
 */
export function firstFrom(matches, order, from) {
  if (matches.length === 0) return -1;
  const rank = (/** @type {number} */ block) => {
    const at = order.indexOf(block);
    return at === -1 ? Number.MAX_SAFE_INTEGER : at;
  };
  const target = rank(from.block);
  const found = matches.findIndex(
    (match) =>
      rank(match.block) > target || (rank(match.block) === target && match.start >= from.offset)
  );
  return found === -1 ? 0 : found;
}

/**
 * Expands `$1`, `$<name>`, `$&` and `$$` the way `String.replace` does.
 *
 * The Milkdown plugin imports this from here, so the expansion rules cannot
 * drift between the two surfaces while both are mounted.
 * @param {RegExpMatchArray} match
 * @param {string} replacement
 * @returns {string}
 */
export function expand(match, replacement) {
  return replacement.replace(/\$(\$|&|\d{1,2}|<[^>]+>)/g, (token, ref) => {
    if (ref === '$') return '$';
    if (ref === '&') return match[0];
    if (ref.startsWith('<')) return match.groups?.[ref.slice(1, -1)] ?? '';
    const index = Number(ref);
    return index > 0 && index < match.length ? (match[index] ?? '') : token;
  });
}

/**
 * The commands that replace `matches` with `replacement`.
 *
 * Back to front, so that an earlier match's offsets still mean what they meant
 * when the scan found them: replacing front to back would shift every offset
 * after the first edit. Sent with `asOneEdit`, the lot is one undo step.
 *
 * A match in a code block is replaced as text like any other; the engine edits
 * code with the same offsets it reports.
 * @param {FindMatch[]} matches
 * @param {string} replacement
 * @param {boolean} useGroups - Expand `$1` (regex mode).
 * @returns {import('$lib/rust-editor.js').EditorCommand[]}
 */
export function replaceCommands(matches, replacement, useGroups) {
  /** @type {import('$lib/rust-editor.js').EditorCommand[]} */
  const commands = [];
  for (let index = matches.length - 1; index >= 0; index -= 1) {
    const match = matches[index];
    const text = useGroups ? expand(match.groups, replacement) : replacement;
    commands.push({
      command: 'setSelection',
      anchor: { block: match.block, offset: match.start },
      head: { block: match.block, offset: match.end },
    });
    // An empty replacement is a delete: `insertText` of nothing is refused by
    // the engine, which has nothing to insert.
    commands.push(text ? { command: 'insertText', text } : { command: 'delete', forward: true });
  }
  return commands;
}
