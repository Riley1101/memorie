import { $inputRule, $prose } from '@milkdown/kit/utils';
import { InputRule } from '@milkdown/kit/prose/inputrules';
import { Plugin, TextSelection } from '@milkdown/kit/prose/state';
import { writingState } from '$lib/runes/writing.svelte.js';

/**
 * A smart-punctuation rule. When `regex` has a capture group only that group
 * is replaced, so the context it matched (a space, an opening bracket) stays.
 * Code blocks and inline code are skipped by ProseMirror itself, and Backspace
 * right after a replacement undoes it (Milkdown's base keymap runs
 * `undoInputRule` first).
 * @param {RegExp} regex
 * @param {string} replacement
 */
function smartRule(regex, replacement) {
  return $inputRule(
    () =>
      new InputRule(regex, (state, match, start, end) => {
        if (!writingState.smartPunctuation) return null;
        if (match[1]) start += match[0].lastIndexOf(match[1]);
        return state.tr.insertText(replacement, start, end);
      })
  );
}

/** Characters after which a quote opens rather than closes. */
const OPENS_AFTER = `(?:^|[\\s{[(<'"‘“—])`;

/**
 * Curly quotes, em dash and ellipsis, the way Typora's "smart punctuation"
 * does it. `--` only converts after other text on the line, so `---` at the
 * start of a line still becomes a divider.
 */
export const smartPunctuation = [
  smartRule(/[^\s-]\s?(--)$/, '—'),
  smartRule(/(\.\.\.)$/, '…'),
  smartRule(new RegExp(`${OPENS_AFTER}(")$`), '“'),
  smartRule(/(")$/, '”'),
  smartRule(new RegExp(`${OPENS_AFTER}(')$`), '‘'),
  smartRule(/(')$/, '’'),
];

/** @type {Record<string, string>} */
const PAIRS = { '(': ')', '[': ']', '{': '}' };
const CLOSERS = new Set(Object.values(PAIRS));

/** An opener only auto-closes when followed by nothing, space or punctuation, so `(` before a word stays single. */
const CLOSE_BEFORE = /^$|^[\s)\]}.,;:!?'"’”]/;

/**
 * The characters either side of an empty selection inside its text block.
 * @param {import('@milkdown/kit/prose/state').EditorState} state
 */
function around(state) {
  const { $from } = state.selection;
  const offset = $from.parentOffset;
  const text = (/** @type {number} */ from, /** @type {number} */ to) =>
    $from.parent.textBetween(from, to, '', '￼');
  return {
    before: text(Math.max(0, offset - 1), offset),
    after: text(offset, Math.min($from.parent.content.size, offset + 1)),
    lineBefore: text(0, offset),
  };
}

/**
 * Auto-closes brackets: typing `(` gives `()` with the caret inside, typing
 * `)` in front of a `)` steps over it, Backspace between an empty pair removes
 * both, and typing an opener with text selected wraps the selection.
 */
export const autoPair = $prose(
  () =>
    new Plugin({
      props: {
        handleTextInput(view, from, to, text) {
          if (!writingState.autoPair || view.composing || text.length !== 1) return false;
          const { state } = view;
          const { selection } = state;
          if (!(selection instanceof TextSelection) || !selection.$from.parent.inlineContent) return false;

          const close = PAIRS[text];
          if (close) {
            if (!selection.empty) {
              if (!selection.$from.sameParent(selection.$to)) return false;
              const tr = state.tr.insertText(close, to).insertText(text, from);
              tr.setSelection(TextSelection.create(tr.doc, from + 1, to + 1));
              view.dispatch(tr);
              return true;
            }
            if (!CLOSE_BEFORE.test(around(state).after)) return false;
            const tr = state.tr.insertText(text + close, from, to);
            tr.setSelection(TextSelection.create(tr.doc, from + 1));
            view.dispatch(tr);
            return true;
          }

          if (CLOSERS.has(text) && selection.empty) {
            const { after, lineBefore } = around(state);
            if (after !== text) return false;
            // Only step over a closer that has an unmatched opener before it on this line.
            const open = Object.keys(PAIRS).find((o) => PAIRS[o] === text) ?? '';
            const depth = [...lineBefore].reduce((n, c) => n + (c === open ? 1 : c === text ? -1 : 0), 0);
            if (depth <= 0) return false;
            view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, from + 1)));
            return true;
          }
          return false;
        },
        handleKeyDown(view, event) {
          if (event.key !== 'Backspace' || !writingState.autoPair) return false;
          const { state } = view;
          if (!state.selection.empty) return false;
          const { before, after } = around(state);
          if (!PAIRS[before] || PAIRS[before] !== after) return false;
          const pos = state.selection.from;
          view.dispatch(state.tr.delete(pos - 1, pos + 1));
          return true;
        },
      },
    })
);
