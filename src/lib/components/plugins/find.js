import { $prose } from '@milkdown/kit/utils';
import { Plugin, PluginKey, TextSelection } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';

/**
 * @file Find and replace in the open writing (⌘F). Highlights every match,
 * marks the current one, and replaces one or all. Queries follow project
 * search: plain text or a regular expression, with match case and whole word.
 *
 * Matches never cross a textblock, the same as a line in grep. While the
 * writer types, matches just move with the text; the full rescan waits until
 * typing pauses.
 */

/**
 * @typedef {{ caseSensitive: boolean, wholeWord: boolean, regex: boolean }} FindOptions
 * @typedef {{ from: number, to: number, groups: RegExpMatchArray }} FindMatch
 * @typedef {{
 *   regex: RegExp | null,
 *   useGroups: boolean,
 *   matches: FindMatch[],
 *   current: number,
 *   set: DecorationSet,
 *   stale: boolean,
 * }} FindState
 * `stale` means the text changed since the last scan: matches were only
 * moved with the edit and may no longer match.
 */

export const findKey = new PluginKey('find');

/**
 * Past this many matches we stop collecting, so a one-letter query in a
 * novel stays responsive.
 */
const MAX_MATCHES = 5000;

/**
 * Pause in typing before matches are rescanned; longer for big documents.
 * @param {import('@milkdown/kit/prose/model').Node} doc
 */
const rescanDelay = (doc) => (doc.content.size > 100_000 ? 600 : 200);

/** Stands in for images and other inline leaves, so offsets equal positions. */
const LEAF = '￼';

/**
 * Builds the search pattern, or throws on a bad regular expression. Whole
 * word counts any letter or digit as part of a word, like the backend's
 * Unicode `\b`; JavaScript's `\b` only knows ASCII, so "café" would never match.
 * @param {string} query
 * @param {FindOptions} options
 */
export function buildFindRegex(query, options) {
  let source = options.regex ? query : query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  if (options.wholeWord) source = `(?<![\\p{L}\\p{N}_])(?:${source})(?![\\p{L}\\p{N}_])`;
  return new RegExp(source, options.caseSensitive ? 'gu' : 'giu');
}

/**
 * Every match in the document's textblocks, in order. Matches that take in
 * an image or other inline node are skipped, so replacing never deletes one.
 * @param {import('@milkdown/kit/prose/model').Node} doc
 * @param {RegExp} regex
 * @param {number} [limit]
 * @returns {FindMatch[]}
 */
export function findAll(doc, regex, limit = MAX_MATCHES) {
  /** @type {FindMatch[]} */
  const matches = [];
  doc.descendants((node, pos) => {
    if (matches.length >= limit) return false;
    if (!node.isTextblock) return true;
    const text = node.textBetween(0, node.content.size, undefined, LEAF);
    for (const m of text.matchAll(regex)) {
      if (!m[0] || m[0].includes(LEAF)) continue;
      const from = pos + 1 + m.index;
      matches.push({ from, to: from + m[0].length, groups: m });
      if (matches.length >= limit) break;
    }
    return false;
  });
  return matches;
}

/**
 * @param {FindMatch} match
 * @param {boolean} isCurrent
 */
function matchDecoration(match, isCurrent) {
  return Decoration.inline(match.from, match.to, {
    class: isCurrent ? 'find-match find-match--current' : 'find-match',
  });
}

/**
 * @param {import('@milkdown/kit/prose/model').Node} doc
 * @param {FindMatch[]} matches
 * @param {number} current
 */
function decorate(doc, matches, current) {
  return DecorationSet.create(
    doc,
    matches.map((m, i) => matchDecoration(m, i === current))
  );
}

/**
 * Redraws one match as current or not, leaving the rest of the set alone.
 * @param {DecorationSet} set
 * @param {import('@milkdown/kit/prose/model').Node} doc
 * @param {FindMatch | undefined} match
 * @param {boolean} isCurrent
 */
function restyle(set, doc, match, isCurrent) {
  if (!match) return set;
  const old = set
    .find(match.from, match.to)
    .filter((d) => d.from === match.from && d.to === match.to);
  return set.remove(old).add(doc, [matchDecoration(match, isCurrent)]);
}

/**
 * Index of the first match at or after `pos`, wrapping to the first.
 * @param {FindMatch[]} matches
 * @param {number} pos
 */
function firstFrom(matches, pos) {
  if (!matches.length) return -1;
  const i = matches.findIndex((m) => m.from >= pos);
  return i === -1 ? 0 : i;
}

/**
 * Expands `$1`, `$<name>`, `$&` and `$$` the way `String.replace` does.
 * @param {RegExpMatchArray} m
 * @param {string} replacement
 */
function expand(m, replacement) {
  return replacement.replace(/\$(\$|&|\d{1,2}|<[^>]+>)/g, (token, ref) => {
    if (ref === '$') return '$';
    if (ref === '&') return m[0];
    if (ref.startsWith('<')) return m.groups?.[ref.slice(1, -1)] ?? '';
    const index = Number(ref);
    return index > 0 && index < m.length ? (m[index] ?? '') : token;
  });
}

/** @returns {FindState} */
function idle() {
  return {
    regex: null,
    useGroups: false,
    matches: [],
    current: -1,
    set: DecorationSet.empty,
    stale: false,
  };
}

/** @param {import('@milkdown/kit/prose/state').EditorState} state */
export function getFindState(state) {
  return /** @type {FindState | undefined} */ (findKey.getState(state));
}

/**
 * Rescans now if the text changed since the last scan, so stepping and
 * replacing never act on a match the writer has since edited.
 * @param {import('@milkdown/kit/prose/view').EditorView} view
 */
function freshen(view) {
  if (getFindState(view.state)?.stale) {
    view.dispatch(view.state.tr.setMeta(findKey, { rescan: true }));
  }
}

/**
 * Selects the current match and scrolls it to the middle of the pane.
 * @param {import('@milkdown/kit/prose/view').EditorView} view
 */
function revealCurrent(view) {
  const find = getFindState(view.state);
  const match = find?.matches[find.current];
  if (!match) return;
  view.dispatch(
    view.state.tr.setSelection(TextSelection.create(view.state.doc, match.from, match.to))
  );
  const { node } = view.domAtPos(match.from);
  const el = node instanceof HTMLElement ? node : node.parentElement;
  el?.scrollIntoView({ block: 'center' });
}

/**
 * Starts (or updates) a search; a `null` regex clears it. The current match
 * becomes the first one at or after the caret.
 * @param {import('@milkdown/kit/prose/view').EditorView} view
 * @param {RegExp | null} regex
 * @param {boolean} useGroups - Expand `$1` in replacements (regex mode).
 */
export function setFind(view, regex, useGroups) {
  if (!regex && !getFindState(view.state)?.regex) return;
  view.dispatch(view.state.tr.setMeta(findKey, { regex, useGroups }));
  if (regex) revealCurrent(view);
}

/**
 * Moves to the next (or previous) match, wrapping around.
 * @param {import('@milkdown/kit/prose/view').EditorView} view
 * @param {1 | -1} dir
 */
export function findStep(view, dir) {
  freshen(view);
  const find = getFindState(view.state);
  if (!find?.matches.length) return;
  const n = find.matches.length;
  const current = (find.current + dir + n) % n;
  view.dispatch(view.state.tr.setMeta(findKey, { current }));
  revealCurrent(view);
}

/**
 * Replaces the current match and moves on to the next one.
 * @param {import('@milkdown/kit/prose/view').EditorView} view
 * @param {string} replacement
 */
export function replaceCurrent(view, replacement) {
  freshen(view);
  const find = getFindState(view.state);
  const match = find?.matches[find.current];
  if (!find || !match) return;
  const text = find.useGroups ? expand(match.groups, replacement) : replacement;
  const tr = text
    ? view.state.tr.insertText(text, match.from, match.to)
    : view.state.tr.delete(match.from, match.to);
  // Carry on from just past the replacement, not from its start.
  tr.setMeta(findKey, { after: tr.mapping.map(match.to) });
  view.dispatch(tr);
  revealCurrent(view);
}

/**
 * Replaces every match in one step, so one undo brings them all back.
 * @param {import('@milkdown/kit/prose/view').EditorView} view
 * @param {string} replacement
 * @returns {number} How many were replaced.
 */
export function replaceAll(view, replacement) {
  freshen(view);
  const find = getFindState(view.state);
  if (!find?.matches.length) return 0;
  const tr = view.state.tr;
  for (let i = find.matches.length - 1; i >= 0; i--) {
    const match = find.matches[i];
    const text = find.useGroups ? expand(match.groups, replacement) : replacement;
    if (text) tr.insertText(text, match.from, match.to);
    else tr.delete(match.from, match.to);
  }
  view.dispatch(tr);
  return find.matches.length;
}

/** @type {Set<(find: FindState) => void>} */
const watchers = new Set();

/**
 * Calls `fn` whenever the matches or the current match change, including
 * from typing in the editor.
 * @param {(find: FindState) => void} fn
 * @returns {() => void} Stops watching.
 */
export function watchFind(fn) {
  watchers.add(fn);
  return () => watchers.delete(fn);
}

/**
 * Moves matches with an edit, dropping any the edit swallowed. Much cheaper
 * than a rescan, which waits until typing pauses.
 * @param {FindState} value
 * @param {import('@milkdown/kit/prose/state').Transaction} tr
 * @param {number | undefined} after - Make the first match from here current.
 * @returns {FindState}
 */
function mapThrough(value, tr, after) {
  const old = value.matches[value.current];
  /** @type {FindMatch[]} */
  const matches = [];
  let kept = -1;
  let set = value.set.map(tr.mapping, tr.doc);
  for (const m of value.matches) {
    const from = tr.mapping.map(m.from, 1);
    const to = tr.mapping.map(m.to, -1);
    // A replaced range maps onto the new text rather than collapsing, so
    // check the text itself: keep the match only if the edit left it alone.
    if (from >= to || tr.doc.textBetween(from, to, undefined, LEAF) !== m.groups[0]) {
      set = set.remove(set.find(from, to).filter((d) => d.from === from && d.to === to));
      continue;
    }
    if (m === old) kept = matches.length;
    matches.push({ from, to, groups: m.groups });
  }
  let current;
  if (after !== undefined) current = firstFrom(matches, after);
  else if (kept !== -1) current = kept;
  else current = Math.min(value.current, matches.length - 1);
  if (current !== kept) {
    set = restyle(set, tr.doc, matches[kept], false);
    set = restyle(set, tr.doc, matches[current], true);
  }
  return { ...value, matches, current, set, stale: true };
}

export const findPlugin = $prose(
  () =>
    new Plugin({
      key: findKey,
      state: {
        init: idle,
        /** @returns {FindState} */
        apply(tr, value, _old, state) {
          const meta = tr.getMeta(findKey);
          if (meta && 'regex' in meta) {
            if (!meta.regex) return idle();
            const matches = findAll(state.doc, meta.regex);
            const current = firstFrom(matches, state.selection.from);
            return {
              regex: meta.regex,
              useGroups: meta.useGroups,
              matches,
              current,
              set: decorate(state.doc, matches, current),
              stale: false,
            };
          }
          if (!value.regex) return value;
          if (tr.docChanged) return mapThrough(value, tr, meta?.after);
          if (meta?.rescan) {
            // Stay on the match the writer was on, or the next one after it.
            const anchor = value.matches[value.current]?.from ?? state.selection.from;
            const matches = findAll(state.doc, value.regex);
            const current = firstFrom(matches, anchor);
            const set = decorate(state.doc, matches, current);
            return { ...value, matches, current, set, stale: false };
          }
          if (meta && 'current' in meta && meta.current !== value.current) {
            let set = restyle(value.set, state.doc, value.matches[value.current], false);
            set = restyle(set, state.doc, value.matches[meta.current], true);
            return { ...value, current: meta.current, set };
          }
          return value;
        },
      },
      props: {
        decorations(state) {
          return getFindState(state)?.set;
        },
      },
      view: () => {
        /** @type {ReturnType<typeof setTimeout> | null} */
        let timer = null;
        return {
          update(view, prevState) {
            const find = getFindState(view.state);
            if (!find || find === getFindState(prevState)) return;
            watchers.forEach((fn) => fn(find));
            // Rescan once typing pauses; each edit pushes it back.
            if (view.state.doc === prevState.doc) return;
            if (timer) clearTimeout(timer);
            timer = null;
            if (!find.stale) return;
            timer = setTimeout(() => {
              timer = null;
              freshen(view);
            }, rescanDelay(view.state.doc));
          },
          destroy() {
            if (timer) clearTimeout(timer);
          },
        };
      },
    })
);
