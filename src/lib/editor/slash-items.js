/**
 * @file What the slash menu offers, and how to find the trigger that opens it.
 *
 * The list is in the engine's command vocabulary, which is the one that
 * survives: Milkdown's adapter maps each item's id to its own commands, the
 * same way the AI box's mark buttons are mapped. Keeping the labels, the
 * keywords and the order in one place is the point — two menus that drift
 * apart are worse than one menu that is missing.
 */

import { fuzzyScore } from '$lib/components/plugins/fuzzy.js';

/** @typedef {import('$lib/rust-editor.js').EditorCommand} EditorCommand */

/**
 * @typedef {Object} SlashItem
 * @property {string} id
 * @property {string} label
 * @property {string} description
 * @property {string[]} keywords
 * @property {EditorCommand[]} [commands] - What the engine runs. Absent for an
 *   item that needs something asked of the writer first.
 * @property {'link'} [asks] - What has to be asked for before it can run.
 */

/**
 * @param {string} modKey - Spelled for this platform, for the link hint.
 * @returns {SlashItem[]}
 */
export function slashItems(modKey = '⌘') {
  return [
    {
      id: 'h1',
      label: 'Heading 1',
      description: 'Big section heading',
      keywords: ['h1', 'heading', 'title'],
      commands: [{ command: 'setHeadingLevel', level: 1 }],
    },
    {
      id: 'h2',
      label: 'Heading 2',
      description: 'Medium section heading',
      keywords: ['h2', 'heading', 'subtitle'],
      commands: [{ command: 'setHeadingLevel', level: 2 }],
    },
    {
      id: 'h3',
      label: 'Heading 3',
      description: 'Small section heading',
      keywords: ['h3', 'heading'],
      commands: [{ command: 'setHeadingLevel', level: 3 }],
    },
    {
      id: 'bold',
      label: 'Bold',
      description: 'Toggle bold at cursor',
      keywords: ['bold', 'strong'],
      commands: [{ command: 'toggleMark', mark: 'bold' }],
    },
    {
      id: 'italic',
      label: 'Italic',
      description: 'Toggle italic at cursor',
      keywords: ['italic', 'emphasis', 'em'],
      commands: [{ command: 'toggleMark', mark: 'italic' }],
    },
    {
      id: 'strikethrough',
      label: 'Strikethrough',
      description: 'Toggle strikethrough at cursor',
      keywords: ['strikethrough', 'strike', 'del'],
      commands: [{ command: 'toggleMark', mark: 'strike' }],
    },
    {
      id: 'inline-code',
      label: 'Inline Code',
      description: 'Toggle inline code at cursor',
      keywords: ['code', 'inline'],
      commands: [{ command: 'toggleMark', mark: 'code' }],
    },
    {
      id: 'bullet-list',
      label: 'Bullet List',
      description: 'Unordered list',
      keywords: ['bullet', 'list', 'ul', 'unordered'],
      commands: [{ command: 'toggleList', ordered: false }],
    },
    {
      id: 'ordered-list',
      label: 'Numbered List',
      description: 'Ordered list',
      keywords: ['number', 'ordered', 'list', 'ol'],
      commands: [{ command: 'toggleList', ordered: true }],
    },
    {
      id: 'blockquote',
      label: 'Quote',
      description: 'Blockquote',
      keywords: ['quote', 'blockquote', 'citation'],
      commands: [{ command: 'wrapInQuote' }],
    },
    {
      id: 'code-block',
      label: 'Code Block',
      description: 'Fenced code block',
      keywords: ['code', 'block', 'fence', 'snippet'],
      commands: [{ command: 'setCodeBlock' }],
    },
    {
      id: 'link',
      label: 'Link',
      description: `Insert a link (${modKey}K links a selection)`,
      keywords: ['link', 'url', 'href', 'web'],
      asks: 'link',
    },
    {
      id: 'divider',
      label: 'Divider',
      description: 'Horizontal rule',
      keywords: ['hr', 'divider', 'rule', 'line'],
      commands: [{ command: 'insertThematicBreak' }],
    },
  ];
}

/**
 * The items matching `query`, best first. No query is every item in order.
 * @param {SlashItem[]} items
 * @param {string} query
 * @returns {SlashItem[]}
 */
export function filterSlashItems(items, query) {
  if (!query) return items;
  return items
    .map((item) => ({
      item,
      best: Math.max(
        ...[item.label, ...item.keywords].map((field) => fuzzyScore(query, field) ?? -1)
      ),
    }))
    .filter(({ best }) => best >= 0)
    .sort((a, b) => b.best - a.best)
    .map(({ item }) => item);
}

/**
 * The slash trigger the caret is sitting after, or null.
 *
 * The `/` has to start the block or follow whitespace, so a date and a path do
 * not open a menu, and the query may not contain whitespace — once the writer
 * types a space they are writing prose, not choosing a command. The returned
 * `start` is where the `/` is, which is what has to be deleted when an item
 * runs.
 * @param {string} blockText
 * @param {number} offset - The caret, in UTF-16 units.
 * @returns {{ query: string, start: number } | null}
 */
export function slashTrigger(blockText, offset) {
  const before = blockText.slice(0, offset);
  const slash = before.lastIndexOf('/');
  if (slash === -1) return null;
  if (slash > 0 && !/\s/.test(before[slash - 1])) return null;
  const query = before.slice(slash + 1);
  if (/\s/.test(query)) return null;
  return { query, start: slash };
}
