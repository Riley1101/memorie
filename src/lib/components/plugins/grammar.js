import { $prose } from '@milkdown/kit/utils';
import { Plugin } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { mount, unmount } from 'svelte';
import GrammarBox from './GrammarBox.svelte';

export const grammarPlugin = $prose((ctx) => {
  return new Plugin({
    state: {
      init(_, { doc }) { return createDecorations(doc); },
      apply(tr, oldState) { return tr.docChanged ? createDecorations(tr.doc) : oldState; },
    },
    props: {
      decorations(state) { return this.getState(state); },
    },
  });
});

function createDecorations(doc) {
  const decos = [];

  doc.descendants((node, pos) => {
    if (node.type.name === 'paragraph' && node.textContent.length < 200 && node.textContent.length > 100) {

      const container = document.createElement('div');
      
      const handleFix = (view, startPos, endPos, newText) => {
        const { tr } = view.state;
        view.dispatch(
          tr.replaceWith(startPos, endPos, view.state.schema.text(newText))
        );
      };

      decos.push(
        Decoration.widget(pos + node.nodeSize, (view) => {
          const component = mount(GrammarBox, {
            target: container,
            props: {
              originalText: node.textContent,
              onFix: (fixedText) => handleFix(view, pos + 1, pos + node.nodeSize, fixedText)
            }
          });
          return container;
        }, {
          side: 1,
          block: true,
        })
      );
    }
    return true;
  });

  return DecorationSet.create(doc, decos);
}
