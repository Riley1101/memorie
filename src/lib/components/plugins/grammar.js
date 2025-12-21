import { $prose } from '@milkdown/kit/utils';
import { Plugin } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { mount } from 'svelte';
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
  
  let sequence = 0;

  doc.descendants((node, pos) => {
    if (node.type.name === 'paragraph') {
      
      if (node.textContent.trim().length === 0) {
        return true; 
      }

      // Match Rust's "chunking" logic
      // Only attach the box if it meets your criteria (< 200 & > 10 chars)
      if (node.textContent.length < 200 && node.textContent.length > 10) {
        const container = document.createElement('div');
        
        const currentSequence = sequence;

        decos.push(
          Decoration.widget(pos + node.nodeSize, (view, getPos) => {
            
            const handleFix = (fixedText) => {
              const currentEnd = getPos(); 
              const currentStart = currentEnd - node.content.size; // Calculate start
              const { tr } = view.state;
              
              view.dispatch(
                tr.replaceWith(currentStart, currentEnd - 1, view.state.schema.text(fixedText))
              );
            };

            mount(GrammarBox, {
              target: container,
              props: {
                sequence: currentSequence,  
                originalText: node.textContent, 
                onFix: handleFix
              }
            });
            return container;
          }, {
            side: 1,
            block: true,
            key: `grammar-${sequence}` 
          })
        );
      }
      sequence++;
    }
    return true;
  });

  return DecorationSet.create(doc, decos);
}
