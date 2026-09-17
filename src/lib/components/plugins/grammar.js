import { $prose } from '@milkdown/kit/utils';
import { Plugin } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { mount } from 'svelte';
import GrammarBox from './GrammarBox.svelte';

/**
 * A Milkdown plugin that adds grammar correction widgets to selected paragraphs.
 * When a paragraph is selected, a GrammarBox widget is displayed at the end of the paragraph,
 * allowing users to suggest corrections.
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
    container.style.position = 'fixed';
    container.style.zIndex = '50';

    /** @param {import("prosemirror-view").EditorView} view */
    const reposition = (view) => {
      const coords = view.coordsAtPos(Math.min(selTo, view.state.doc.content.size));
      container.style.top = `${coords.bottom + 8}px`;
      container.style.left = `${coords.left}px`;
    };

    decos.push(
      Decoration.widget(
        nodeEnd,
        (view, getPos) => {
          const handleFix = (fixedText) => {
            const { tr } = view.state;
            const start = (getPos() || 0) - node.nodeSize + 1;
            const end = (getPos() || 0) - 1;
            view.dispatch(tr.replaceWith(start, end, view.state.schema.text(fixedText)));
          };

          reposition(view);
          const onScrollOrResize = () => reposition(view);
          window.addEventListener('scroll', onScrollOrResize, { capture: true, passive: true });
          window.addEventListener('resize', onScrollOrResize);
          // Listener teardown is parked on the node so the decoration's
          // `destroy` can reach it.
          /** @type {HTMLDivElement & { _cleanup?: () => void }} */ (container)._cleanup = () => {
            window.removeEventListener('scroll', onScrollOrResize, { capture: true });
            window.removeEventListener('resize', onScrollOrResize);
          };

          mount(GrammarBox, {
            target: container,
            props: {
              sequence: 0,
              originalText: node.textContent,
              onFix: handleFix,
              view,
            },
          });
          return container;
        },
        {
          side: 1,
          // Include the selection range in the key so the widget (and its
          // position) is recreated whenever the selection changes, even when
          // it stays within the same paragraph.
          key: `grammar-${pos}-${selFrom}-${selTo}`,
          destroy: () =>
            /** @type {HTMLDivElement & { _cleanup?: () => void }} */ (container)._cleanup?.(),
        }
      )
    );
  }

  return DecorationSet.create(doc, decos);
}
