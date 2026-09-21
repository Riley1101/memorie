/**
 * Builds the transaction that swaps a paragraph's text for an AI suggestion.
 *
 * `paragraphEnd` is the position just after the paragraph (where the grammar
 * widget sits). The paragraph is read from the live document rather than
 * captured when the widget was made, since the widget can outlive edits to it.
 *
 * Blank lines in the suggestion become separate paragraphs; single line breaks
 * collapse to spaces, as they would in the saved markdown anyway.
 *
 * @param {import('prosemirror-state').EditorState} state
 * @param {number} paragraphEnd
 * @param {string} suggestion
 * @returns {import('prosemirror-state').Transaction | null} null when there is nothing to apply.
 */
export function applySuggestion(state, paragraphEnd, suggestion) {
  const paragraph = state.doc.resolve(paragraphEnd).nodeBefore;
  if (!paragraph || paragraph.type.name !== 'paragraph') return null;

  const parts = (suggestion ?? '')
    .split(/\n\s*\n/)
    .map((part) => part.replace(/\s*\n\s*/g, ' ').trim())
    .filter(Boolean);
  // An empty suggestion would need an empty text node, which ProseMirror rejects.
  if (parts.length === 0) return null;

  const paragraphs = parts.map((part) =>
    paragraph.type.create(paragraph.attrs, state.schema.text(part))
  );
  const start = paragraphEnd - paragraph.nodeSize;

  return state.tr.replaceWith(start, paragraphEnd, paragraphs).scrollIntoView();
}
