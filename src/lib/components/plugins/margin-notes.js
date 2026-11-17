import { $prose } from '@milkdown/kit/utils';
import { Plugin, PluginKey, TextSelection } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { notesState } from '$lib/runes/notes.svelte.js';
import { toast } from '$lib/toast.js';
import { textOf, indexText, openView } from './doc-text.js';

/** @typedef {import('@milkdown/kit/prose/model').Node} PMNode */

const key = new PluginKey('margin-notes');

/** How much text before a note is remembered, to tell repeated phrases apart. */
const PREFIX_LENGTH = 32;

/** @param {PMNode} doc @param {number} from */
function prefixOf(doc, from) {
  return textOf(doc, Math.max(0, from - PREFIX_LENGTH * 2), from).slice(-PREFIX_LENGTH);
}

/**
 * Finds where each note belongs: after its prefix when that still matches,
 * else the first place its quote appears.
 * @param {PMNode} doc
 * @param {import('$lib/runes/notes.svelte.js').Note[]} notes
 */
function locate(doc, notes) {
  const { text, posAt } = indexText(doc);
  /** @type {{ id: string, from: number, to: number }[]} */
  const found = [];
  for (const note of notes) {
    let at = note.prefix ? text.indexOf(note.prefix + note.quote) : -1;
    if (at >= 0) at += note.prefix.length;
    else at = text.indexOf(note.quote);
    if (at < 0) continue;
    const from = posAt(at);
    const last = posAt(at + note.quote.length - 1);
    if (from === null || last === null) continue;
    found.push({ id: note.id, from, to: last + 1 });
  }
  return found;
}

/** @param {string} id @param {number} from @param {number} to */
function noteDecoration(id, from, to) {
  return Decoration.inline(from, to, { class: 'margin-note', 'data-note': id }, { id });
}

/**
 * @typedef {{ set: DecorationSet, active: string | null }} NotesPluginState
 * @typedef {{ place?: { id: string, from: number, to: number }[], add?: { id: string, from: number, to: number }, remove?: string }} NotesMeta
 */

/**
 * Highlights the text under each margin note, follows it through edits, and
 * tells `notesState` where the notes are and which one holds the caret.
 */
export const marginNotesPlugin = $prose(() => {
  return new Plugin({
    key,
    state: {
      /** @returns {NotesPluginState} */
      init: () => ({ set: DecorationSet.empty, active: null }),
      /** @param {import('@milkdown/kit/prose/state').Transaction} tr @param {NotesPluginState} value */
      apply(tr, value) {
        /** @type {NotesMeta | undefined} */
        const meta = tr.getMeta(key);
        let set = value.set.map(tr.mapping, tr.doc);
        if (meta?.place) set = DecorationSet.create(tr.doc, meta.place.map((n) => noteDecoration(n.id, n.from, n.to)));
        if (meta?.add) set = set.add(tr.doc, [noteDecoration(meta.add.id, meta.add.from, meta.add.to)]);
        if (meta?.remove) {
          const id = meta.remove;
          set = set.remove(set.find(undefined, undefined, (spec) => spec.id === id));
        }
        if (set === value.set && !tr.selectionSet) return value;
        const head = tr.selection.head;
        const active = set.find(head, head)[0]?.spec.id ?? null;
        // Typing with no notes, or moving the caret within one, changes nothing here.
        if (set === value.set && active === value.active) return value;
        return { set, active };
      },
    },
    props: {
      decorations(state) {
        const { set, active } = /** @type {NotesPluginState} */ (key.getState(state));
        if (!active) return set;
        const current = set.find(undefined, undefined, (spec) => spec.id === active)[0];
        if (!current) return set;
        return set.add(state.doc, [
          Decoration.inline(current.from, current.to, { class: 'margin-note-active' }, { id: `${active}:active` }),
        ]);
      },
    },
    view(view) {
      // The writing's notes were loaded with its metadata just before the
      // editor was made; place them once the view exists.
      queueMicrotask(() => {
        if (view.isDestroyed) return;
        const place = locate(view.state.doc, notesState.list);
        view.dispatch(view.state.tr.setMeta(key, { place }).setMeta('addToHistory', false));
      });

      /** Reports where the notes are now. */
      function report() {
        const { set, active } = /** @type {NotesPluginState} */ (key.getState(view.state));
        const { doc } = view.state;
        /** @type {Record<string, { from: number, to: number, quote: string, prefix: string }>} */
        const placed = {};
        for (const d of set.find()) {
          const id = d.spec.id;
          if (placed[id]) continue;
          placed[id] = { from: d.from, to: d.to, quote: textOf(doc, d.from, d.to), prefix: prefixOf(doc, d.from) };
        }
        notesState.sync(placed, active);
      }

      return {
        update(view, prev) {
          const now = key.getState(view.state);
          if (now === key.getState(prev) && prev.doc === view.state.doc) return;
          report();
        },
        destroy() {
          notesState.anchors = {};
          notesState.activeId = null;
        },
      };
    },
  });
});

/**
 * Leaves a new note on the selected text. The note's card then opens for
 * the writer to type it.
 * @returns {string | null} The new note's id.
 */
export function addNoteOnSelection() {
  const view = openView();
  if (!view) return null;
  const { state } = view;
  let { from, to } = state.selection;
  const raw = textOf(state.doc, from, to);
  // A note sits on words, not on the spaces or line breaks around them.
  const lead = raw.length - raw.trimStart().length;
  const trail = raw.length - raw.trimEnd().length;
  from += lead;
  to -= trail;
  const quote = textOf(state.doc, from, to);
  if (from >= to || !quote.trim()) {
    toast.info('Select some text to leave a note on');
    return null;
  }
  const note = notesState.create({ quote, prefix: prefixOf(state.doc, from) });
  view.dispatch(
    state.tr
      .setMeta(key, { add: { id: note.id, from, to } })
      .setSelection(TextSelection.create(state.doc, to))
      .setMeta('addToHistory', false)
  );
  return note.id;
}

/**
 * Removes a note and its highlight.
 * @param {string} id
 */
export async function deleteNote(id) {
  const view = openView();
  if (view) view.dispatch(view.state.tr.setMeta(key, { remove: id }).setMeta('addToHistory', false));
  await notesState.remove(id);
}

/**
 * Writes a note's text, or removes it (and its highlight) when left empty.
 * @param {string} id
 * @param {string} text
 */
export async function saveNote(id, text) {
  if (!text.trim()) return deleteNote(id);
  await notesState.write(id, text);
}

/**
 * Selects the text a note is on and brings it into view.
 * @param {string} id
 */
export function revealNote(id) {
  const view = openView();
  const at = notesState.anchors[id];
  if (!view || !at) return;
  const tr = view.state.tr.setSelection(TextSelection.create(view.state.doc, at.from, at.to));
  view.dispatch(tr.scrollIntoView());
  view.focus();
}

/**
 * The page-relative top of each placed note's first line, for the margin.
 * @param {HTMLElement} surface - The element the margin is laid out against.
 * @returns {{ id: string, top: number }[]}
 */
export function measureNotes(surface) {
  const view = openView();
  if (!view) return [];
  const base = surface.getBoundingClientRect().top;
  const size = view.state.doc.content.size;
  return Object.entries(notesState.anchors)
    .map(([id, at]) => {
      try {
        return { id, top: view.coordsAtPos(Math.min(at.from, size), 1).top - base };
      } catch {
        return null;
      }
    })
    .filter((n) => n !== null)
    .sort((a, b) => a.top - b.top);
}
