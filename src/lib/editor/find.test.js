/// <reference types="bun" />
/**
 * @file Tests for find and replace against the engine.
 *
 * The matching and the offset arithmetic are what break, so they are tested
 * without a DOM or an engine: `findAll` reads the blocks the view already
 * holds, and `replaceCommands` only builds commands.
 */

import { describe, expect, test } from 'bun:test';
import {
  buildFindRegex,
  findAll,
  findDecorations,
  firstFrom,
  expand,
  replaceCommands,
} from './find.js';

/** @typedef {import('$lib/rust-editor.js').EditorBlock} EditorBlock */

/** @param {number} id @param {string} text @returns {EditorBlock} */
const para = (id, text) => ({
  id,
  kind: 'paragraph',
  runs: [{ type: 'text', text, length: text.length }],
});

const plain = { caseSensitive: false, wholeWord: false, regex: false };

describe('buildFindRegex', () => {
  test('plain text is escaped, so punctuation is literal', () => {
    expect(buildFindRegex('a.b', plain).test('axb')).toBe(false);
    expect(buildFindRegex('a.b', plain).test('a.b')).toBe(true);
  });

  test('case is ignored unless asked for', () => {
    expect(buildFindRegex('cat', plain).test('CAT')).toBe(true);
    expect(buildFindRegex('cat', { ...plain, caseSensitive: true }).test('CAT')).toBe(false);
  });

  test('whole word knows about letters outside ASCII', () => {
    // JavaScript's own \b would fail this: é is not an ASCII word character,
    // so "café" would match inside "cafés".
    const regex = buildFindRegex('café', { ...plain, wholeWord: true });
    expect(regex.test('a café here')).toBe(true);
    expect(buildFindRegex('café', { ...plain, wholeWord: true }).test('cafés')).toBe(false);
  });

  test('regex mode passes the pattern through', () => {
    expect(buildFindRegex('ca+t', { ...plain, regex: true }).test('caaat')).toBe(true);
  });
});

describe('findAll', () => {
  test('finds matches in document order with block offsets', () => {
    const blocks = [para(1, 'cat and cat'), para(2, 'a cat')];
    expect(findAll(blocks, buildFindRegex('cat', plain))).toMatchObject([
      { block: 1, start: 0, end: 3 },
      { block: 1, start: 8, end: 11 },
      { block: 2, start: 2, end: 5 },
    ]);
  });

  test('walks into list items and quotes', () => {
    const blocks = [
      { id: 1, kind: 'quote', children: [para(2, 'quoted cat')] },
      { id: 3, kind: 'list', items: [{ blocks: [para(4, 'listed cat')] }] },
    ];
    expect(
      findAll(/** @type {EditorBlock[]} */ (blocks), buildFindRegex('cat', plain))
    ).toMatchObject([
      { block: 2, start: 7 },
      { block: 4, start: 7 },
    ]);
  });

  test('searches a code block as text', () => {
    const blocks = /** @type {EditorBlock[]} */ ([
      { id: 9, kind: 'codeBlock', text: 'let cat = 1;' },
    ]);
    expect(findAll(blocks, buildFindRegex('cat', plain))).toMatchObject([{ block: 9, start: 4 }]);
  });

  test('a match that swallows an image is skipped', () => {
    // Replacing it would delete the image, and the writer asked to replace
    // text. The atom counts as one unit so the offsets after it still line up.
    const blocks = /** @type {EditorBlock[]} */ ([
      {
        id: 1,
        kind: 'paragraph',
        runs: [
          { type: 'text', text: 'a', length: 1 },
          { type: 'image', length: 1, source: 'x.png' },
          { type: 'text', text: 'b and ab', length: 8 },
        ],
      },
    ]);
    expect(findAll(blocks, buildFindRegex('ab', plain))).toMatchObject([{ start: 8, end: 10 }]);
  });

  test('keeps to the limit, so one letter in a novel stays responsive', () => {
    expect(findAll([para(1, 'aaaaa')], buildFindRegex('a', plain), 3)).toHaveLength(3);
  });

  test('a pattern that can match nothing still terminates', () => {
    expect(findAll([para(1, 'abc')], buildFindRegex('x*', { ...plain, regex: true }))).toEqual([]);
  });
});

describe('firstFrom', () => {
  const matches = [
    { block: 1, start: 0, end: 3, groups: /** @type {any} */ (['cat']) },
    { block: 2, start: 5, end: 8, groups: /** @type {any} */ (['cat']) },
    { block: 2, start: 9, end: 12, groups: /** @type {any} */ (['cat']) },
  ];
  const order = [1, 2];

  test('the first match at or after the caret', () => {
    expect(firstFrom(matches, order, { block: 2, offset: 6 })).toBe(2);
  });

  test('a caret before everything finds the first', () => {
    expect(firstFrom(matches, order, { block: 1, offset: 0 })).toBe(0);
  });

  test('a caret past the last match wraps round', () => {
    expect(firstFrom(matches, order, { block: 2, offset: 99 })).toBe(0);
  });

  test('document order, not block id order', () => {
    // Ids say nothing about position: a block pasted in later carries a higher
    // id than the ones below it. With block 2 first in the document, a caret in
    // block 1 is past both of block 2's matches, so the one in block 1 is next.
    const reversed = [
      { block: 2, start: 5, end: 8, groups: /** @type {any} */ (['cat']) },
      { block: 2, start: 9, end: 12, groups: /** @type {any} */ (['cat']) },
      { block: 1, start: 0, end: 3, groups: /** @type {any} */ (['cat']) },
    ];
    expect(firstFrom(reversed, [2, 1], { block: 1, offset: 0 })).toBe(2);
    expect(firstFrom(reversed, [2, 1], { block: 2, offset: 0 })).toBe(0);
  });

  test('no matches is no current match', () => {
    expect(firstFrom([], order, { block: 1, offset: 0 })).toBe(-1);
  });
});

describe('expand', () => {
  const match = /** @type {any} */ (['John Smith', 'John', 'Smith']);
  match.groups = { first: 'John' };

  test('numbered groups, the whole match, and a literal dollar', () => {
    expect(expand(match, '$2, $1')).toBe('Smith, John');
    expect(expand(match, '[$&]')).toBe('[John Smith]');
    expect(expand(match, '$$1')).toBe('$1');
  });

  test('named groups', () => {
    expect(expand(match, 'Hi $<first>')).toBe('Hi John');
  });

  test('a group that does not exist is left as written', () => {
    expect(expand(match, '$9')).toBe('$9');
  });
});

describe('replaceCommands', () => {
  const matches = [
    { block: 1, start: 0, end: 3, groups: /** @type {any} */ (['cat']) },
    { block: 1, start: 8, end: 11, groups: /** @type {any} */ (['cat']) },
  ];

  test('back to front, so earlier offsets stay valid', () => {
    const commands = replaceCommands(matches, 'dog', false);
    expect(commands.map((c) => c.command)).toEqual([
      'setSelection',
      'insertText',
      'setSelection',
      'insertText',
    ]);
    // The later match is replaced first.
    expect(commands[0]).toMatchObject({ anchor: { offset: 8 }, head: { offset: 11 } });
    expect(commands[2]).toMatchObject({ anchor: { offset: 0 }, head: { offset: 3 } });
  });

  test('an empty replacement deletes rather than inserting nothing', () => {
    // `insertText` of an empty string is refused by the engine: there is
    // nothing to insert.
    expect(replaceCommands(matches, '', false).map((c) => c.command)).toEqual([
      'setSelection',
      'delete',
      'setSelection',
      'delete',
    ]);
  });

  test('expands groups only in regex mode', () => {
    const one = [{ block: 1, start: 0, end: 3, groups: /** @type {any} */ (['cat', 'c']) }];
    expect(replaceCommands(one, '[$1]', true)[1]).toMatchObject({ text: '[c]' });
    expect(replaceCommands(one, '[$1]', false)[1]).toMatchObject({ text: '[$1]' });
  });
});

describe('findDecorations', () => {
  test('marks the current match apart from the rest', () => {
    const matches = [
      { block: 1, start: 0, end: 3, groups: /** @type {any} */ (['cat']) },
      { block: 2, start: 0, end: 3, groups: /** @type {any} */ (['cat']) },
    ];
    expect(findDecorations(matches, 1).map((d) => d.class)).toEqual([
      'deco-find',
      'deco-find-current',
    ]);
  });
});
