import { $prose } from '@milkdown/kit/utils';
import { Plugin, PluginKey } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { writingState } from '$lib/runes/writing.svelte.js';

/**
 * Nearest ancestor that actually scrolls vertically.
 * @param {HTMLElement | null} el
 * @returns {HTMLElement | null}
 */
function scrollParent(el) {
  for (let node = el?.parentElement; node; node = node.parentElement) {
    const { overflowY } = getComputedStyle(node);
    if ((overflowY === 'auto' || overflowY === 'scroll') && node.scrollHeight > node.clientHeight) {
      return node;
    }
  }
  return null;
}

/**
 * Scrolls so the caret sits in the vertical middle of the writing area.
 * @param {import('@milkdown/kit/prose/view').EditorView} view
 * @param {ScrollBehavior} [behavior]
 */
export function centerCaret(view, behavior = 'smooth') {
  const scroller = scrollParent(view.dom);
  if (!scroller) return;
  const caret = view.coordsAtPos(view.state.selection.head);
  const box = scroller.getBoundingClientRect();
  const offset = caret.top - (box.top + box.height / 2);
  if (Math.abs(offset) < 4) return;
  scroller.scrollBy({ top: offset, behavior });
}

/**
 * Focus mode support. Marks the top-level block holding the caret so CSS can
 * dim everything else, and keeps the caret centred while focus mode is on
 * (typewriter scrolling). Both are inert unless the page is in focus mode.
 *
 * Only typing and keyboard movement recentre; a click or drag-selection
 * leaves the page where it is, so the text doesn't jump under the pointer.
 */
export const focusPlugin = $prose(() => {
  let pointerSelecting = false;
  return new Plugin({
    key: new PluginKey('focus'),
    props: {
      handleDOMEvents: {
        mousedown() {
          pointerSelecting = true;
          return false;
        },
        keydown() {
          pointerSelecting = false;
          return false;
        },
      },
      decorations(state) {
        const { $head } = state.selection;
        if ($head.depth === 0) return DecorationSet.empty;
        const start = $head.before(1);
        const node = state.doc.child($head.index(0));
        return DecorationSet.create(state.doc, [
          Decoration.node(start, start + node.nodeSize, { class: 'is-current-block' }),
        ]);
      },
    },
    view() {
      return {
        update(view, prev) {
          if (!writingState.focusMode || !view.hasFocus()) return;
          // Reference checks: a deep `eq` would walk the whole document per keystroke.
          const typed = prev.doc !== view.state.doc;
          const moved = prev.selection !== view.state.selection;
          if (typed || (moved && !pointerSelecting)) centerCaret(view);
        },
      };
    },
  });
});
