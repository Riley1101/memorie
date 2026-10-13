/**
 * @file Tests for the decoration layer.
 *
 * `bun test`. These run in Bun rather than in a browser because nothing here
 * touches the DOM: `decorations.js` is deliberately pure so that the part of
 * the view with the fiddly arithmetic in it can be tested without one.
 */

import { describe, expect, test } from 'bun:test';
import { decorate, matchDecorations, textOfRuns, forBlock, flattenBlocks } from './decorations.js';

/** "She found the letter", with `letter` bold. */
const runs = [
  { type: 'text', text: 'She found the ', length: 14 },
  { type: 'text', text: 'letter', length: 6, marks: ['bold'] },
];

const shape = (list) => list.map((run) => [run.text, run.deco]);

describe('textOfRuns', () => {
  test('reads the runs as the engine counts them', () => {
    expect(textOfRuns(runs)).toBe('She found the letter');
  });

  test('an atom is one unit of something', () => {
    expect(textOfRuns([{ type: 'text', text: 'ab' }, { type: 'image' }])).toBe('ab￼');
  });
});

describe('decorate', () => {
  test('a highlight inside one run splits it in three', () => {
    expect(decorate(runs, [{ block: 1, start: 4, end: 9, class: 'deco-find' }])).toEqual([
      { type: 'text', text: 'She ', length: 4 },
      { type: 'text', text: 'found', length: 5, deco: 'deco-find' },
      { type: 'text', text: ' the ', length: 5 },
      { type: 'text', text: 'letter', length: 6, marks: ['bold'] },
    ]);
  });

  test('a highlight across a run boundary splits both', () => {
    expect(shape(decorate(runs, [{ block: 1, start: 10, end: 17, class: 'deco-find' }]))).toEqual([
      ['She found ', undefined],
      ['the ', 'deco-find'],
      ['let', 'deco-find'],
      ['ter', undefined],
    ]);
  });

  test('the split preserves what dom.js measures by', () => {
    const split = decorate(runs, [{ block: 1, start: 4, end: 9, class: 'deco-find' }]);
    expect(split.reduce((total, run) => total + run.length, 0)).toBe(20);
    expect(split.map((run) => run.text).join('')).toBe('She found the letter');
    expect(split.filter((run) => run.marks).map((run) => run.text)).toEqual(['letter']);
  });

  test('no decorations hands back the very same array', () => {
    // Not just an equal one: the view re-renders per keystroke, and copying
    // every run of every block to change nothing is the cost this avoids.
    expect(decorate(runs, [])).toBe(runs);
    expect(decorate(runs, [{ block: 1, start: 3, end: 3, class: 'deco-find' }])).toBe(runs);
  });

  test('an atom is covered whole, never split', () => {
    const withAtom = [
      { type: 'text', text: 'ab', length: 2 },
      { type: 'image', length: 1, source: 'x.png' },
      { type: 'text', text: 'cd', length: 2 },
    ];
    expect(
      decorate(withAtom, [{ block: 1, start: 1, end: 4, class: 'deco-find' }]).map((run) => [
        run.type,
        run.deco,
      ])
    ).toEqual([
      ['text', undefined],
      ['text', 'deco-find'],
      ['image', 'deco-find'],
      ['text', 'deco-find'],
      ['text', undefined],
    ]);
  });

  test('two decorations over the same text both land', () => {
    expect(
      shape(
        decorate([{ type: 'text', text: 'abcdef', length: 6 }], [
          { block: 1, start: 0, end: 4, class: 'deco-find' },
          { block: 1, start: 2, end: 6, class: 'deco-mention' },
        ])
      )
    ).toEqual([
      ['ab', 'deco-find'],
      ['cd', 'deco-find deco-mention'],
      ['ef', 'deco-mention'],
    ]);
  });
});

describe('matchDecorations', () => {
  const blocks = [
    { id: 1, kind: 'paragraph', runs: [{ type: 'text', text: 'cat and cat', length: 11 }] },
    {
      id: 2,
      kind: 'list',
      items: [
        { blocks: [{ id: 3, kind: 'paragraph', runs: [{ type: 'text', text: 'a cat', length: 5 }] }] },
      ],
    },
  ];

  test('finds every match, nested blocks included', () => {
    expect(matchDecorations(blocks, /cat/g, 'deco-find').map((d) => [d.block, d.start, d.end])).toEqual(
      [
        [1, 0, 3],
        [1, 8, 11],
        [3, 2, 5],
      ]
    );
  });

  test('a pattern that can match nothing still terminates', () => {
    expect(matchDecorations(blocks, /x*/g, 'deco-find')).toEqual([]);
  });

  test('reads a code block from its text rather than its runs', () => {
    const code = [{ id: 7, kind: 'codeBlock', text: 'let cat = 1;' }];
    expect(matchDecorations(code, /cat/g, 'deco-find')).toEqual([
      { block: 7, start: 4, end: 7, class: 'deco-find' },
    ]);
  });

  test('flattenBlocks walks quotes, list items and table cells', () => {
    expect(
      flattenBlocks([
        { id: 1, children: [{ id: 2 }] },
        { id: 3, items: [{ blocks: [{ id: 4 }] }] },
        { id: 5, rows: [{ cells: [[{ id: 6 }]] }] },
      ]).map((block) => block.id)
    ).toEqual([1, 2, 3, 4, 5, 6]);
  });
});

describe('forBlock', () => {
  test('keeps only the decorations a block has to draw', () => {
    const all = [
      { block: 1, start: 0, end: 1, class: 'a' },
      { block: 2, start: 0, end: 1, class: 'b' },
    ];
    expect(forBlock(all, 2)).toEqual([{ block: 2, start: 0, end: 1, class: 'b' }]);
  });
});
