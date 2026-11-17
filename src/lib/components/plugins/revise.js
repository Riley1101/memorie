import { $prose } from '@milkdown/kit/utils';
import { Plugin, PluginKey, TextSelection } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { parserCtx } from '@milkdown/kit/core';
import { invoke } from '@tauri-apps/api/core';
import { diffWordsWithSpace } from 'diff';
import { editorState } from '$lib/runes/editor.svelte.js';
import { reviseState } from '$lib/runes/revise.svelte.js';
import { parseWriting } from '$lib/front-matter.js';
import { toast } from '$lib/toast.js';
import { LEAF, textOf, indexText, openView } from './doc-text.js';

/**
 * @file Revise mode: shows, word by word, how the page differs from an
 * earlier version. Added words are underlined, removed ones are shown struck
 * through where they were, and each change can be kept or put back.
 *
 * The earlier version is read as plain text the same way the page is
 * (doc-text.js), so formatting marks never show up as changes. Only the
 * writing's body is compared; front matter (status, notes) is left out.
 */

/** @typedef {import('@milkdown/kit/prose/model').Node} PMNode */

const key = new PluginKey('revise');

/** Quiet time after typing before the page is compared again, in ms. */
const RECOMPUTE_DELAY = 250;

/**
 * @typedef {Object} Change
 * @property {string} id
 * @property {number} from - Where the change starts on the page.
 * @property {number} to - Where it ends; equal to `from` when words were only removed.
 * @property {number} addFrom - The added words, without line breaks at either end.
 * @property {number} addTo
 * @property {number} oldStart - Where it starts in the earlier text.
 * @property {number} oldEnd
 * @property {string} removed - The earlier wording; '' when words were only added.
 */

/**
 * @typedef {{ on: boolean, base: string, changes: Change[], active: string | null, fresh: boolean, set: DecorationSet }} RevisePluginState
 * @typedef {{ start?: { base: string, changes: Change[] }, base?: string, changes?: Change[], stop?: true }} ReviseMeta
 */

/** @type {RevisePluginState} */
const OFF = { on: false, base: '', changes: [], active: null, fresh: true, set: DecorationSet.empty };

/**
 * The changes between the earlier text and the page, in page order.
 * @param {PMNode} doc
 * @param {string} base - The earlier version's text.
 * @returns {Change[]}
 */
function compare(doc, base) {
  const { text, boundaryAt } = indexText(doc);

  /** @type {{ oldStart: number, oldEnd: number, newStart: number, newEnd: number }[]} */
  const runs = [];
  let oldAt = 0;
  let newAt = 0;
  /** @type {typeof runs[number] | null} */
  let open = null;
  for (const part of diffWordsWithSpace(base, text)) {
    const length = part.value.length;
    if (!part.added && !part.removed) {
      if (open) runs.push(open);
      open = null;
      oldAt += length;
      newAt += length;
      continue;
    }
    if (!open) {
      const prev = runs[runs.length - 1];
      // Words changed on both sides of a single space read as one change:
      // reopen the change before when only a space or two sits between.
      if (prev && /^[ \t]{1,2}$/.test(base.slice(prev.oldEnd, oldAt))) {
        open = /** @type {typeof runs[number]} */ (runs.pop());
      } else {
        open = { oldStart: oldAt, oldEnd: oldAt, newStart: newAt, newEnd: newAt };
      }
    }
    if (part.removed) {
      oldAt += length;
      open.oldEnd = oldAt;
    } else {
      newAt += length;
      open.newEnd = newAt;
    }
  }
  if (open) runs.push(open);

  /** @type {Change[]} */
  const changes = [];
  for (const run of runs) {
    const removed = base.slice(run.oldStart, run.oldEnd);
    const added = text.slice(run.newStart, run.newEnd);
    // A paragraph split or joined, or spacing alone, isn't a change in wording.
    if (!removed.trim() && !added.trim()) continue;
    let s = run.newStart;
    let e = run.newEnd;
    while (s < e && text[s] === '\n') s++;
    while (e > s && text[e - 1] === '\n') e--;
    changes.push({
      id: `${run.oldStart}:${run.newStart}`,
      from: boundaryAt(run.newStart),
      to: boundaryAt(run.newEnd),
      addFrom: boundaryAt(s),
      addTo: boundaryAt(e),
      oldStart: run.oldStart,
      oldEnd: run.oldEnd,
      removed,
    });
  }
  return changes;
}

/** @param {string} label @param {() => void} run */
function actionButton(label, run) {
  const button = document.createElement('button');
  button.type = 'button';
  button.textContent = label;
  // Keep the caret (and the change it's on) where it is.
  button.addEventListener('mousedown', (e) => e.preventDefault());
  button.addEventListener('click', run);
  return button;
}

/** @param {Change} change */
function actionsWidget(change) {
  return () => {
    const anchor = document.createElement('span');
    anchor.className = 'revise-actions';
    anchor.contentEditable = 'false';
    const pop = document.createElement('span');
    pop.className = 'revise-actions-pop glass';
    pop.append(
      actionButton('Keep', () => keepChange(change.id)),
      actionButton(change.removed.trim() ? 'Put back' : 'Remove', () => putBack(change.id))
    );
    anchor.append(pop);
    return anchor;
  };
}

/** @param {string} removed @param {boolean} active */
function removedWidget(removed, active) {
  return () => {
    const span = document.createElement('span');
    span.className = active ? 'revise-removed revise-active' : 'revise-removed';
    span.textContent = removed.replaceAll('\n', ' ¶ ');
    return span;
  };
}

/**
 * @param {PMNode} doc
 * @param {Change[]} changes
 * @param {string | null} active
 */
function decorate(doc, changes, active) {
  /** @type {Decoration[]} */
  const decorations = [];
  for (const change of changes) {
    const isActive = change.id === active;
    if (change.removed.trim()) {
      decorations.push(
        Decoration.widget(change.from, removedWidget(change.removed, isActive), {
          side: -1,
          ignoreSelection: true,
          key: `r:${change.id}:${isActive}:${change.removed}`,
        })
      );
    }
    if (change.addTo > change.addFrom) {
      decorations.push(
        Decoration.inline(change.addFrom, change.addTo, {
          class: isActive ? 'revise-added revise-active' : 'revise-added',
        })
      );
    }
    if (isActive) {
      decorations.push(
        Decoration.widget(change.to, actionsWidget(change), {
          side: 1,
          ignoreSelection: true,
          stopEvent: () => true,
          key: `a:${change.id}:${change.removed}`,
        })
      );
    }
  }
  return DecorationSet.create(doc, decorations);
}

/** @param {Change[]} changes @param {number} head */
function changeAt(changes, head) {
  return changes.find((c) => head >= c.from && head <= c.to)?.id ?? null;
}

/** @param {import('@milkdown/kit/prose/state').EditorState} state @returns {RevisePluginState} */
function stateOf(state) {
  return /** @type {RevisePluginState} */ (key.getState(state));
}

export const revisePlugin = $prose(() => {
  return new Plugin({
    key,
    state: {
      init: () => OFF,
      /** @param {import('@milkdown/kit/prose/state').Transaction} tr @param {RevisePluginState} value */
      apply(tr, value) {
        /** @type {ReviseMeta | undefined} */
        const meta = tr.getMeta(key);
        if (meta?.stop) return OFF;
        if (!value.on && !meta?.start) return value;

        let { base, changes, fresh } = value;
        if (meta?.start) {
          base = meta.start.base;
          changes = meta.start.changes;
          fresh = true;
        }
        if (meta?.base !== undefined) base = meta.base;
        if (meta?.changes) {
          changes = meta.changes;
          fresh = true;
        } else if (tr.docChanged && !meta?.start) {
          // Follow the edit until the page is compared again.
          const m = tr.mapping;
          changes = changes.map((c) => {
            const from = m.map(c.from, -1);
            const addFrom = m.map(c.addFrom, 1);
            return {
              ...c,
              from,
              to: Math.max(from, m.map(c.to, 1)),
              addFrom,
              addTo: Math.max(addFrom, m.map(c.addTo, -1)),
            };
          });
          fresh = false;
        }

        const active = changeAt(changes, tr.selection.head);
        if (value.on && changes === value.changes && active === value.active && base === value.base) {
          return value;
        }
        return { on: true, base, changes, active, fresh, set: decorate(tr.doc, changes, active) };
      },
    },
    props: {
      decorations: (state) => stateOf(state).set,
    },
    view() {
      /** @type {ReturnType<typeof setTimeout> | undefined} */
      let timer;
      return {
        update(view, prev) {
          const now = stateOf(view.state);
          const before = stateOf(prev);
          if (now !== before) {
            reviseState.open = now.on;
            reviseState.count = now.changes.length;
            reviseState.position = now.active ? now.changes.findIndex((c) => c.id === now.active) + 1 : 0;
          }
          if (!now.on || now.fresh || prev.doc === view.state.doc) return;
          clearTimeout(timer);
          timer = setTimeout(() => {
            if (view.isDestroyed) return;
            const current = stateOf(view.state);
            if (!current.on) return;
            const changes = compare(view.state.doc, current.base);
            view.dispatch(view.state.tr.setMeta(key, { changes }).setMeta('addToHistory', false));
          }, RECOMPUTE_DELAY);
        },
        destroy() {
          clearTimeout(timer);
          reviseState.reset();
        },
      };
    },
  });
});

/**
 * Starts comparing the page with one saved version.
 * @param {string} fileName
 * @param {number} nodeId - The version, as numbered in the history.
 * @param {string} label - What it is, in words, for the bar ("the start of today").
 */
export async function compareWithVersion(fileName, nodeId, label) {
  /** @type {string | null} */
  let raw;
  try {
    raw = await invoke('get_file_version', { name: fileName, nodeId });
  } catch (e) {
    toast.error('Could not read that version', e);
    return;
  }
  const view = openView();
  if (raw === null || raw === undefined || !view) {
    toast.error('Could not read that version');
    return;
  }
  /** @type {string | null} */
  let base = null;
  try {
    editorState.editor?.action((ctx) => {
      base = indexText(ctx.get(parserCtx)(parseWriting(raw).body)).text;
    });
  } catch (e) {
    console.error('Could not read version for comparing:', e);
  }
  if (base === null) {
    toast.error('Could not read that version');
    return;
  }
  reviseState.label = label;
  const changes = compare(view.state.doc, base);
  view.dispatch(view.state.tr.setMeta(key, { start: { base, changes } }).setMeta('addToHistory', false));
  if (changes.length === 0) toast.info(`No changes since ${label}`);
}

/**
 * Compares the page with how it was when today began: the last version
 * saved before midnight, or the first version when it was started today.
 * @param {string} fileName
 */
export async function compareWithStartOfToday(fileName) {
  /** @type {import('$lib/components/editor-history.svelte').History | null} */
  let history;
  try {
    history = await invoke('get_file_history', { name: fileName });
  } catch (e) {
    toast.error('Could not read the history', e);
    return;
  }
  if (!history || history.current === null) {
    toast.info('No saved versions yet', 'Versions appear after the first save.');
    return;
  }
  const midnight = new Date().setHours(0, 0, 0, 0);
  /** @type {number} */
  let at = history.current;
  let label = 'the start of today';
  for (;;) {
    const node = history.nodes[at];
    if (!node) break;
    if (node.createdAt !== null && node.createdAt !== undefined && node.createdAt < midnight) break;
    if (node.parent === null) {
      label = 'the first version';
      break;
    }
    at = node.parent;
  }
  await compareWithVersion(fileName, at, label);
}

/** Stops comparing. */
export function stopRevise() {
  const view = openView();
  if (view && stateOf(view.state).on) {
    view.dispatch(view.state.tr.setMeta(key, { stop: true }).setMeta('addToHistory', false));
  }
  reviseState.reset();
}

/**
 * Accepts a change: the earlier text takes on the page's wording there, so
 * it stops showing as a change.
 * @param {string} id
 */
export function keepChange(id) {
  const view = openView();
  if (!view) return;
  const { base, changes } = stateOf(view.state);
  const change = changes.find((c) => c.id === id);
  if (!change) return;
  const { doc } = view.state;
  const next = base.slice(0, change.oldStart) + textOf(doc, change.from, change.to) + base.slice(change.oldEnd);
  view.dispatch(
    view.state.tr.setMeta(key, { base: next, changes: compare(doc, next) }).setMeta('addToHistory', false)
  );
  view.focus();
}

/**
 * Undoes a change on the page: the earlier wording goes back in its place.
 * Its words come back; their formatting (bold, links) doesn't.
 * @param {string} id
 */
export function putBack(id) {
  const view = openView();
  if (!view) return;
  const { base, changes } = stateOf(view.state);
  const change = changes.find((c) => c.id === id);
  if (!change) return;
  const tr = view.state.tr.delete(change.from, change.to);
  let pos = change.from;
  change.removed
    .replaceAll(LEAF, '')
    .split('\n')
    .forEach((piece, i) => {
      if (i > 0) {
        tr.split(pos);
        pos += 2;
      }
      if (piece) {
        tr.insertText(piece, pos);
        pos += piece.length;
      }
    });
  tr.setSelection(TextSelection.create(tr.doc, pos));
  view.dispatch(tr.setMeta(key, { changes: compare(tr.doc, base) }));
  view.focus();
}

/**
 * Moves the caret to the next or previous change.
 * @param {1 | -1} direction
 */
export function goToChange(direction) {
  const view = openView();
  if (!view) return;
  const { changes, active } = stateOf(view.state);
  if (changes.length === 0) return;
  const head = view.state.selection.head;
  const current = changes.find((c) => c.id === active);
  const target =
    direction > 0
      ? (changes.find((c) => c.from > (current ? current.to : head)) ?? changes[0])
      : (changes.findLast((c) => c.to < (current ? current.from : head)) ?? changes[changes.length - 1]);
  const tr = view.state.tr.setSelection(TextSelection.create(view.state.doc, target.from));
  view.dispatch(tr.scrollIntoView());
  view.focus();
}
