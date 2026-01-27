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
 * Creates decorations for selected paragraphs in the document.
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

  let sequence = 0;

  doc.descendants((node, pos) => {
    if (node.type.name === 'paragraph') {
      if (node.textContent.trim().length === 0) return true;

      const nodeEnd = pos + node.nodeSize;
      const isSelected = selFrom !== null && pos < selTo && nodeEnd > selFrom;

      if (isSelected) {
        const container = document.createElement('div');
        const currentSequence = sequence;

        decos.push(
          Decoration.widget(
            nodeEnd,
            (view, getPos) => {
              /**
               * Handles the fix action from the GrammarBox.
               * @param {string} fixedText - The corrected text to replace the original paragraph content.
               */
              const handleFix = (fixedText) => {
                const { tr } = view.state;

                const start = (getPos() || 0) - node.nodeSize + 1;
                const end = (getPos() || 0) - 1;

                view.dispatch(tr.replaceWith(start, end, view.state.schema.text(fixedText)));
              };
              mount(GrammarBox, {
                target: container,
                props: {
                  sequence: currentSequence,
                  originalText: node.textContent,
                  onFix: handleFix,
                },
              });
              return container;
            },
            {
              side: 1,
              block: true,
              key: `grammar-${pos}`,
            }
          )
        );
      }
      sequence++;
    }
    return true;
  });

  return DecorationSet.create(doc, decos);
}
