import { $prose } from '@milkdown/kit/utils';
import { Plugin, PluginKey } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';

const PLACEHOLDER_TEXT = 'Start writing…';

/**
 * A Milkdown plugin that shows placeholder text when the document is empty.
 */
export const placeholderPlugin = $prose(() => {
  return new Plugin({
    key: new PluginKey('placeholder'),
    props: {
      decorations(state) {
        const { doc } = state;
        const isEmpty =
          doc.childCount === 1 &&
          doc.firstChild.isTextblock &&
          doc.firstChild.content.size === 0;

        if (!isEmpty) return DecorationSet.empty;

        return DecorationSet.create(doc, [
          Decoration.node(0, doc.firstChild.nodeSize, {
            class: 'is-empty',
            'data-placeholder': PLACEHOLDER_TEXT,
          }),
        ]);
      },
    },
  });
});
