import { $prose } from '@milkdown/kit/utils';
import { Plugin } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { mount, unmount } from 'svelte';
import GrammarBox from './GrammarBox.svelte';
import { applySuggestion } from './apply-suggestion.js';

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
