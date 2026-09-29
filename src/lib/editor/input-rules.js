/**
 * @file What a keystroke means beyond "insert this character": Markdown
 * shortcuts, smart punctuation and bracket pairing.
 *
 * These are pure functions from context to commands, so they can be reasoned
 * about without a DOM and reused by any view. The rules mirror what the
 * Milkdown editor does today (`plugins/typing.js` and the commonmark input
 * rules), so switching editors doesn't change how typing feels.
 */

/** @typedef {import('$lib/rust-editor.js').EditorCommand} EditorCommand */

/**
 * @typedef {Object} TypingContext
 * @property {number} block - The block the caret is in.
 * @property {number} start - Selection start, in UTF-16 units.
 * @property {number} end - Selection end; equal to `start` when collapsed.
 * @property {string} text - The block's text as the view has it.
 * @property {boolean} smartPunctuation
 * @property {boolean} autoPair
 */

/** Opening bracket or quote to its closing partner. */
const PAIRS = /** @type {Record<string, string>} */ ({
  '(': ')',
  '[': ']',
  '{': '}',
  '"': '"',
  "'": "'",
  '`': '`',
});

/** @param {number} block @param {number} start @param {number} end */
function select(block, start, end) {
  return /** @type {EditorCommand} */ ({
    command: 'setSelection',
    anchor: { block, offset: start },
    head: { block, offset: end },
  });
}

/**
 * The commands for typing `character`, or null to insert it as it is.
 *
 * Rules are tried in the order a writer would expect: a Markdown shortcut
 * turns the line into a block, a bracket pairs or wraps, and punctuation is
 * tidied. Only one applies.
 * @param {string} character
 * @param {TypingContext} context
 * @returns {EditorCommand[] | null}
 */
export function commandsForCharacter(character, context) {
  return (
    markdownShortcut(character, context) ??
    bracketPair(character, context) ??
    smartPunctuation(character, context)
  );
}

/**
 * `## ` becomes a heading, `- ` a list, `> ` a quote, ``` a code block.
 *
 * The prefix the writer typed is deleted and the block changes kind, which is
 * why this is a command list rather than a text substitution.
 * @param {string} character
 * @param {TypingContext} context
 * @returns {EditorCommand[] | null}
 */
function markdownShortcut(character, { block, start, end, text }) {
  if (start !== end) return null;
  const before = text.slice(0, start);

  // A fence anywhere on an otherwise empty line, not just at the start.
  if (character === '`' && before === '``') {
    return [select(block, 0, 2), { command: 'delete', forward: true }, { command: 'setCodeBlock' }];
  }
  if (character !== ' ') return null;

  /** @type {EditorCommand | null} */
  let change = null;
  if (/^#{1,6}$/.test(before)) {
    change = { command: 'setHeadingLevel', level: before.length };
  } else if (/^[-*+]$/.test(before)) {
    change = { command: 'toggleList', ordered: false };
  } else if (/^\d{1,9}[.)]$/.test(before)) {
    change = { command: 'toggleList', ordered: true };
  } else if (before === '>') {
    change = { command: 'wrapInQuote' };
  } else if (/^[-*+] \[[ xX]?$/.test(before)) {
    // `- [ ` on a list item: make it a task.
    change = { command: 'toggleTask' };
  }
  if (!change) return null;

  return [select(block, 0, start), { command: 'delete', forward: true }, change];
}

/**
 * An opening bracket wraps the selection, or is inserted with its partner and
 * the caret between them. A closing bracket typed over the one just added is
 * stepped past rather than doubled.
 * @param {string} character
 * @param {TypingContext} context
 * @returns {EditorCommand[] | null}
 */
function bracketPair(character, { block, start, end, text, autoPair }) {
  if (!autoPair) return null;
  const closing = PAIRS[character];

  if (start !== end) {
    // Wrapping a selection: quotes and brackets both read as "put this around
    // what I picked".
    if (!closing) return null;
    return [
      select(block, start, end),
      { command: 'insertText', text: `${character}${text.slice(start, end)}${closing}` },
      select(block, start + 1, end + 1),
    ];
  }

  // Typing the closing half of a pair the editor just added: step over it.
  if (Object.values(PAIRS).includes(character) && text.slice(start, start + 1) === character) {
    return [select(block, start + 1, start + 1)];
  }
  if (!closing) return null;
  // A quote between word characters is an apostrophe, not an opening quote.
  if (
    (character === '"' || character === "'" || character === '`') &&
    /\w$/.test(text.slice(0, start))
  ) {
    return null;
  }
  return [
    select(block, start, start),
    { command: 'insertText', text: `${character}${closing}` },
    select(block, start + 1, start + 1),
  ];
}

/**
 * Curly quotes, an em dash from `--`, an ellipsis from `...`.
 * @param {string} character
 * @param {TypingContext} context
 * @returns {EditorCommand[] | null}
 */
function smartPunctuation(character, { block, start, end, text, smartPunctuation: on }) {
  if (!on || start !== end) return null;
  const before = text.slice(0, start);

  if (character === '-' && before.endsWith('-') && !before.endsWith('--')) {
    return [select(block, start - 1, start), { command: 'insertText', text: '—' }];
  }
  if (character === '.' && before.endsWith('..') && !before.endsWith('...')) {
    return [select(block, start - 2, start), { command: 'insertText', text: '…' }];
  }
  if (character === '"' || character === "'") {
    // Opening after a space, a bracket or nothing at all; closing otherwise.
    const opening = before === '' || /[\s([{‘“]$/.test(before);
    const quote = character === '"' ? (opening ? '“' : '”') : opening ? '‘' : '’';
    return [select(block, start, start), { command: 'insertText', text: quote }];
  }
  return null;
}

/**
 * Whether a character could start a rule, so the caller can skip the work of
 * gathering context for ordinary letters.
 * @param {string} character
 */
export function mightBeRule(character) {
  return character.length === 1 && ' `([{"\'-.)]}'.includes(character);
}
