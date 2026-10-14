/// <reference types="bun" />
/**
 * @file Tests for the `[[` trigger.
 *
 * The bracket arithmetic is the part that breaks: auto-pairing puts `]]` after
 * the caret, so the range that gets replaced is not the range the writer typed.
 */

import { describe, expect, test } from 'bun:test';
import { docRefTrigger, docRefHref, stripMd, DOC_REF_PREFIX } from './doc-ref.js';

describe('docRefTrigger', () => {
  test('opens on an unclosed double bracket', () => {
    expect(docRefTrigger('[[', 2)).toEqual({ query: '', start: 0, end: 2 });
    expect(docRefTrigger('[[chap', 6)).toEqual({ query: 'chap', start: 0, end: 6 });
  });

  test('takes in the brackets auto-pairing left after the caret', () => {
    // The writer typed `[[`, which became `[[]]` with the caret in the middle,
    // then typed `chap`. Replacing has to swallow the closing pair as well.
    expect(docRefTrigger('[[chap]]', 6)).toEqual({ query: 'chap', start: 0, end: 8 });
  });

  test('without a closing pair the range ends at the caret', () => {
    expect(docRefTrigger('[[chap] more', 7)).toBeNull();
  });

  test('opens mid-sentence', () => {
    expect(docRefTrigger('see [[ch', 8)).toEqual({ query: 'ch', start: 4, end: 8 });
  });

  test('a single bracket is not a trigger', () => {
    expect(docRefTrigger('[chap', 5)).toBeNull();
  });

  test('a closed reference is not a trigger any more', () => {
    expect(docRefTrigger('[[chap]] and', 12)).toBeNull();
  });

  test('reads from the caret, not the end of the block', () => {
    expect(docRefTrigger('[[chap and more', 4)).toEqual({ query: 'ch', start: 0, end: 4 });
  });

  test('no brackets is no trigger', () => {
    expect(docRefTrigger('just prose', 10)).toBeNull();
  });
});

describe('docRefHref', () => {
  test('encodes the name, so a space or a slash survives the round trip', () => {
    expect(docRefHref('Novel/Ch 1.md')).toBe(`${DOC_REF_PREFIX}Novel%2FCh%201.md`);
  });
});

describe('stripMd', () => {
  test('drops the extension, whatever its case', () => {
    expect(stripMd('Chapter 1.md')).toBe('Chapter 1');
    expect(stripMd('Chapter 1.MD')).toBe('Chapter 1');
    expect(stripMd('no-extension')).toBe('no-extension');
  });

  test('a trailing space defeats the strip, since the trim comes after it', () => {
    // Longstanding, and shared with the Milkdown path. A file name with a
    // trailing space is pathological and the only consequence is an uglier
    // menu entry, so it is recorded here rather than changed.
    expect(stripMd(' Chapter 1.md ')).toBe('Chapter 1.md');
  });
});
