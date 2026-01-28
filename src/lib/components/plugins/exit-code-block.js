import { $prose } from '@milkdown/kit/utils';
import { Plugin, TextSelection } from '@milkdown/kit/prose/state';

/**
 * A Milkdown plugin that allows escaping from a code block by pressing ArrowDown
 * at the end of the block when it's the last node in the document.
 */
export const exitCodeBlockPlugin = $prose(() => {
  return new Plugin({
    props: {
      handleKeyDown(view, event) {
        if (event.key !== 'ArrowDown') return false;

        const { state, dispatch } = view;
        const { selection, doc } = state;
        const { $from, empty } = selection;

        if (!empty) return false;

        // Check if we are inside a code block
        const node = $from.node($from.depth);
        if (node.type.name !== 'code_block') return false;

        // Check if we are at the very end of the code block
        const isAtEndOfBlock = $from.parentOffset === node.content.size;
        if (!isAtEndOfBlock) return false;

        // Check if this code block is the last node in the document
        const isAtEndOfDoc = $from.after($from.depth) === doc.content.size;
        if (!isAtEndOfDoc) return false;

        // If we are at the end of a code block at the end of the doc, 
        // insert a new paragraph and move selection there.
        if (dispatch) {
          const type = state.schema.nodes.paragraph;
          const tr = state.tr.insert($from.after($from.depth), type.createAndFill());
          dispatch(tr.setSelection(TextSelection.create(tr.doc, tr.selection.$from.after($from.depth) + 1)));
        }

        return true;
      }
    }
  });
});
