import { $prose } from '@milkdown/kit/utils';
import { Plugin } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { mount, unmount } from 'svelte';
import GrammarBox from './GrammarBox.svelte';
import { applySuggestion } from './apply-suggestion.js';

/**
 * A Milkdown plugin that shows a toolbar over a text selection: formatting,
 * a link, a margin note, the codex, and AI rewrites of the paragraph when AI
 * is enabled (GrammarBox decides). It sits above the selection, where it
 * covers text already read, and drops below it near the top of the window.
 *
 * @returns {Plugin} - The Milkdown plugin for grammar correction.
 */
export const grammarPlugin = $prose(() => {
  return new Plugin({
    state: {
      init(_, { doc }) {
        return createSelectionDecoration(doc);
      },
      apply(tr, oldSet) {
        if (!tr.selectionSet && !tr.docChanged) return oldSet;

        const { selection, doc } = tr;

        if (selection.empty) {
          return DecorationSet.empty;
        }

        return createSelectionDecoration(doc, selection.from, selection.to);
      },
    },
    props: {
      decorations(state) {
        return this.getState(state);
      },
    },
  });
});

/**
 * Creates a decoration for the last selected paragraph only (single AI box per selection).
 *
 * @param {import("prosemirror-model").Node} doc - The ProseMirror document node.
 * @param {number} selFrom - The start position of the selection.
 * @param {number} selTo - The end position of the selection.
 * @returns {DecorationSet} - The set of decorations for the selected paragraphs.
 */
function createSelectionDecoration(doc, selFrom = 0, selTo = 0) {
  /**
   * @type {Decoration[]}
   */
  const decos = [];

  /** @type {{ node: import("prosemirror-model").Node, pos: number } | null} */
  let lastSelected = null;

  doc.descendants((node, pos) => {
    if (node.type.name === 'paragraph') {
      if (node.textContent.trim().length === 0) return true;

      const nodeEnd = pos + node.nodeSize;
      const isSelected = selFrom !== null && pos < selTo && nodeEnd > selFrom;

      if (isSelected) {
        lastSelected = { node, pos };
      }
      return true;
    }
    return true;
  });

  if (lastSelected) {
    const { node, pos } = lastSelected;
    const nodeEnd = pos + node.nodeSize;
    const container = document.createElement('div');
    container.className = 'group/box';
    container.style.position = 'fixed';
    container.style.zIndex = '50';

    /** Below this distance from the top of the window there is no room above the selection. */
    const ROOM_ABOVE = 120;

    /** @param {import("prosemirror-view").EditorView} view */
    const reposition = (view) => {
      const size = view.state.doc.content.size;
      const start = view.coordsAtPos(Math.min(selFrom, size));
      const end = view.coordsAtPos(Math.min(selTo, size));
      const above = start.top >= ROOM_ABOVE;
      container.dataset.placement = above ? 'above' : 'below';
      container.style.top = above ? `${start.top - 8}px` : `${end.bottom + 8}px`;
      container.style.transform = above ? 'translateY(-100%)' : '';
      container.style.left = `${Math.max(8, Math.min(start.left, window.innerWidth - 8 - container.offsetWidth))}px`;
    };

    decos.push(
      Decoration.widget(
        nodeEnd,
        (view, getPos) => {
          const handleFix = (fixedText) => {
            const pos = getPos();
            if (pos == null) return;
            const tr = applySuggestion(view.state, pos, fixedText);
            if (!tr) return;
            view.dispatch(tr);
            view.focus();
          };

          reposition(view);
          const onScrollOrResize = () => reposition(view);
          window.addEventListener('scroll', onScrollOrResize, { capture: true, passive: true });
          window.addEventListener('resize', onScrollOrResize);
          // Listener teardown is parked on the node so the decoration's
          // `destroy` can reach it.
          const component = mount(GrammarBox, {
            target: container,
            props: {
              sequence: 0,
              originalText: node.textContent,
              onFix: handleFix,
              view,
            },
          });
          // Its width is known only once it is on the page; keep it inside the window.
          requestAnimationFrame(() => reposition(view));
          /** @type {HTMLDivElement & { _cleanup?: () => void }} */ (container)._cleanup = () => {
            window.removeEventListener('scroll', onScrollOrResize, { capture: true });
            window.removeEventListener('resize', onScrollOrResize);
            unmount(component);
          };
          return container;
        },
        {
          side: 1,
          // Include the selection range in the key so the widget (and its
          // position) is recreated whenever the selection changes, even when
          // it stays within the same paragraph.
          key: `grammar-${pos}-${selFrom}-${selTo}`,
          // The box is UI, not document: keep the editor from treating clicks
          // in it as caret moves, which would collapse the selection and tear
          // the box down before Apply's click lands.
          stopEvent: () => true,
          ignoreSelection: true,
          destroy: () =>
            /** @type {HTMLDivElement & { _cleanup?: () => void }} */ (container)._cleanup?.(),
        }
      )
    );
  }

  return DecorationSet.create(doc, decos);
}
