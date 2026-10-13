/// <reference types="bun" />
/**
 * @file Tests for codex mention matching.
 *
 * The rules mirror the backend's `codex::Matcher`, so the cases that matter are
 * the ones where a naive regex gets it wrong: overlapping names, case modes,
 * and word boundaries around letters outside ASCII.
 */

import { describe, expect, test } from 'bun:test';
import { buildMatcher, findMentions, mentionDecorations } from './mentions.js';

/**
 * @param {string} name
 * @param {Partial<import('$lib/runes/codex.svelte.js').Entity>} [rest]
 */
const entity = (name, rest = {}) =>
  /** @type {import('$lib/runes/codex.svelte.js').Entity} */ ({
    name,
    aliases: [],
    kind: 'character',
    matchMode: 'case-sensitive',
    summary: '',
    ...rest,
  });

/**
 * @param {number} id
 * @param {string} text
 * @returns {import('$lib/rust-editor.js').EditorBlock}
 */
const para = (id, text) => ({
  id,
  kind: 'paragraph',
  runs: [{ type: 'text', text, length: text.length }],
});

describe('findMentions', () => {
  test('the longest name wins where two overlap', () => {
    const matcher = buildMatcher([entity('Citadel'), entity('Iron Citadel')]);
    expect(findMentions(matcher, 'the Iron Citadel fell')).toMatchObject([
      { from: 4, to: 16, entity: { name: 'Iron Citadel' } },
    ]);
  });

  test('whole words only', () => {
    const matcher = buildMatcher([entity('Kael')]);
    expect(findMentions(matcher, 'Kaelen went')).toEqual([]);
    expect(findMentions(matcher, 'Kael went')).toHaveLength(1);
  });

  test('word boundaries hold around letters outside ASCII', () => {
    const matcher = buildMatcher([entity('Zoë')]);
    expect(findMentions(matcher, 'Zoë spoke')).toHaveLength(1);
    expect(findMentions(matcher, 'Zoës')).toEqual([]);
  });

  test('each entry keeps its own case mode', () => {
    const matcher = buildMatcher([
      entity('Rose', { matchMode: 'case-sensitive' }),
      entity('thorn', { matchMode: 'insensitive' }),
    ]);
    expect(findMentions(matcher, 'rose and Thorn')).toMatchObject([{ entity: { name: 'thorn' } }]);
  });

  test('an entry set to off is never matched', () => {
    expect(findMentions(buildMatcher([entity('Kael', { matchMode: 'off' })]), 'Kael')).toEqual([]);
  });

  test('aliases match as the entry does', () => {
    const matcher = buildMatcher([entity('Kaelen Dor', { aliases: ['Kael'] })]);
    expect(findMentions(matcher, 'Kael arrived')).toMatchObject([
      { entity: { name: 'Kaelen Dor' } },
    ]);
  });

  test('a one-character name is ignored, as too noisy to underline', () => {
    expect(buildMatcher([entity('X')]).patterns).toEqual([]);
  });

  test('mentions come back in order and never overlap', () => {
    const matcher = buildMatcher([entity('Kael'), entity('Dor')]);
    const hits = findMentions(matcher, 'Kael met Dor');
    expect(hits.map((hit) => hit.from)).toEqual([0, 9]);
  });
});

describe('mentionDecorations', () => {
  test('carries the kind and the summary for the hover', () => {
    const entities = [entity('Kael', { summary: 'A smith', kind: 'character' })];
    expect(mentionDecorations([para(1, 'Kael waited')], entities)).toEqual([
      {
        block: 1,
        start: 0,
        end: 4,
        class: 'deco-mention',
        attrs: {
          'data-codex-kind': 'character',
          'data-codex-name': 'Kael',
          title: 'Kael: A smith',
        },
      },
    ]);
  });

  test('a name inside a code block is code, not a mention', () => {
    const blocks = /** @type {import('$lib/rust-editor.js').EditorBlock[]} */ ([
      { id: 1, kind: 'codeBlock', text: 'let Kael = 1;' },
    ]);
    expect(mentionDecorations(blocks, [entity('Kael')])).toEqual([]);
  });

  test('no entries means no work and no decorations', () => {
    expect(mentionDecorations([para(1, 'Kael')], [])).toEqual([]);
  });

  test('walks into quotes and list items', () => {
    const blocks = /** @type {import('$lib/rust-editor.js').EditorBlock[]} */ ([
      { id: 1, kind: 'quote', children: [para(2, 'Kael spoke')] },
      { id: 3, kind: 'list', items: [{ blocks: [para(4, 'Kael again')] }] },
    ]);
    expect(mentionDecorations(blocks, [entity('Kael')]).map((d) => d.block)).toEqual([2, 4]);
  });
});
