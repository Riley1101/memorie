/// <reference types="bun" />
/**
 * @file Tests for the slash menu's trigger and filtering.
 *
 * The trigger is where a slash menu goes wrong: it has to open on a `/` the
 * writer meant as a command and stay shut for the ones in dates, paths and
 * fractions.
 */

import { describe, expect, test } from 'bun:test';
import { slashItems, filterSlashItems, slashTrigger } from './slash-items.js';

const items = slashItems('⌘');

describe('slashTrigger', () => {
  test('opens on a slash at the start of a block', () => {
    expect(slashTrigger('/', 1)).toEqual({ query: '', start: 0 });
    expect(slashTrigger('/head', 5)).toEqual({ query: 'head', start: 0 });
  });

  test('opens on a slash after a space', () => {
    expect(slashTrigger('text /head', 10)).toEqual({ query: 'head', start: 5 });
  });

  test('stays shut inside a date, a path or a fraction', () => {
    expect(slashTrigger('12/05/2026', 10)).toBeNull();
    expect(slashTrigger('notes/scene.md', 14)).toBeNull();
    expect(slashTrigger('1/2', 3)).toBeNull();
  });

  test('closes once the query takes a space, because that is prose', () => {
    expect(slashTrigger('/head ', 6)).toBeNull();
    expect(slashTrigger('/head and more', 14)).toBeNull();
  });

  test('reads from the caret, not the end of the block', () => {
    // The writer typed `/h`, then moved back a character.
    expect(slashTrigger('/head', 2)).toEqual({ query: 'h', start: 0 });
  });

  test('takes the slash nearest the caret', () => {
    expect(slashTrigger('/one /two', 9)).toEqual({ query: 'two', start: 5 });
  });

  test('no slash at all is no trigger', () => {
    expect(slashTrigger('just prose', 10)).toBeNull();
  });
});

describe('filterSlashItems', () => {
  test('no query is every item, in the order they are written', () => {
    expect(filterSlashItems(items, '')).toEqual(items);
  });

  test('matches a label', () => {
    expect(filterSlashItems(items, 'quote')[0].id).toBe('blockquote');
  });

  test('matches a keyword the label does not contain', () => {
    expect(filterSlashItems(items, 'ul')[0].id).toBe('bullet-list');
    // `hr` reaches the divider, but not first: it scores the same against the
    // link's `href` — same first letter, same second — and the sort is stable,
    // so the one written earlier wins the tie. The Milkdown menu has always
    // ranked these two the same way; the scorer is shared.
    expect(filterSlashItems(items, 'hr').map((item) => item.id)).toContain('divider');
  });

  test('a query that matches nothing matches nothing', () => {
    expect(filterSlashItems(items, 'zzzz')).toEqual([]);
  });
});

describe('slashItems', () => {
  test('every item either runs commands or says what it has to ask for', () => {
    for (const item of items) {
      expect(Boolean(item.commands) !== Boolean(item.asks)).toBe(true);
    }
  });

  test('ids are unique, since the Milkdown adapter maps by them', () => {
    const ids = items.map((item) => item.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  test('the link hint names this platform', () => {
    expect(slashItems('Ctrl-').find((item) => item.id === 'link')?.description).toContain('Ctrl-K');
  });
});
