/**
 * @file Doc references: `[[` to link one writing to another.
 *
 * The prefix, the candidate list and the trigger live here rather than in the
 * Milkdown plugin — the fourth thing to make that trip after the find regex,
 * the `$1` expansion and the mention matcher, for the same two reasons. Both
 * surfaces have to agree on what a reference looks like, since they read each
 * other's files, and the engine path should not load ProseMirror to offer a
 * list of writings.
 */

import { fuzzyScore } from '$lib/components/plugins/fuzzy.js';
import { dirOf, baseOf } from '$lib/paths.js';

/**
 * Prefix marking a link's href as an internal doc reference rather than an
 * external URL. Deliberately scheme-less (no "protocol:") — Milkdown's link
 * mark strips any href with an unrecognized scheme when rendering to the DOM.
 */
export const DOC_REF_PREFIX = '#writing/';

/**
 * @typedef {Object} DocRefItem
 * @property {string} name - Content-relative file name, e.g. "Novel/Ch 1.md".
 * @property {string} title - Display title, no folder, no ".md".
 * @property {string} folder - Parent folder, or "" for top-level.
 */

/** @param {string} name */
export function stripMd(name) {
  return name.replace(/\.md$/i, '').trim();
}

/**
 * Writings as candidates. The file list is passed in rather than read from
 * `fileManager` here, so that everything in this module is a function of its
 * arguments and the trigger arithmetic can be tested without the app's state.
 * @param {{ name: string }[]} files
 * @returns {DocRefItem[]}
 */
export function docRefItemsFrom(files) {
  return files.map((file) => ({
    name: file.name,
    title: stripMd(baseOf(file.name)),
    folder: dirOf(file.name),
  }));
}

/**
 * The candidates matching `query`, best first, capped so a big project still
 * draws a menu rather than a wall.
 * @param {DocRefItem[]} items
 * @param {string} query
 * @returns {DocRefItem[]}
 */
export function filterDocRefItems(items, query) {
  if (!query) return items.slice(0, 20);
  return items
    .map((item) => ({ item, score: fuzzyScore(query, item.title) }))
    .filter(({ score }) => score !== null)
    .sort((a, b) => (b.score ?? 0) - (a.score ?? 0))
    .slice(0, 20)
    .map(({ item }) => item);
}

/** The href a reference to `name` is written with. @param {string} name */
export function docRefHref(name) {
  return `${DOC_REF_PREFIX}${encodeURIComponent(name)}`;
}

/** Matches an unclosed `[[query` at the end of the text before the caret. */
const TRIGGER_RE = /\[\[([^[\]]*)$/;

/**
 * The `[[` trigger the caret is sitting after, or null.
 *
 * `start` is where the `[[` begins and `end` takes in the `]]` that auto-pairing
 * leaves after the caret, so running an item replaces the whole thing rather
 * than leaving two brackets behind.
 * @param {string} blockText
 * @param {number} offset - The caret, in UTF-16 units.
 * @returns {{ query: string, start: number, end: number } | null}
 */
export function docRefTrigger(blockText, offset) {
  const match = TRIGGER_RE.exec(blockText.slice(0, offset));
  if (!match) return null;
  const closing = blockText.slice(offset, offset + 2) === ']]' ? 2 : 0;
  return {
    query: match[1],
    start: offset - (match[1].length + 2),
    end: offset + closing,
  };
}
