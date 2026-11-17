import { $prose } from '@milkdown/kit/utils';
import { Plugin, PluginKey } from '@milkdown/kit/prose/state';

/**
 * Reports document and selection changes to the editor component. Stands in
 * for Milkdown's listener plugin, which also serializes the whole document to
 * Markdown when the editor opens and deep-compares documents after every pause
 * in typing; Memoire uses neither.
 * @param {{
 *   onDoc: (doc: import('@milkdown/kit/prose/model').Node) => void,
 *   onSelection: (state: import('@milkdown/kit/prose/state').EditorState) => void,
 * }} handlers
 */
export function changesPlugin({ onDoc, onSelection }) {
  return $prose(
    () =>
      new Plugin({
        key: new PluginKey('memoire-changes'),
        view: () => ({
          update(view, prevState) {
            const { state } = view;
            if (state.doc !== prevState.doc) onDoc(state.doc);
            if (!state.selection.eq(prevState.selection)) onSelection(state);
          },
        }),
      })
  );
}
