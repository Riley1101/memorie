import { $prose as prose } from '@milkdown/kit/utils';
import { commandsCtx, editorViewCtx } from '@milkdown/kit/core';
import { SlashProvider } from '@milkdown/kit/plugin/slash';
import { Plugin } from '@milkdown/kit/prose/state';
import {
  wrapInHeadingCommand,
  wrapInBlockquoteCommand,
  createCodeBlockCommand,
  insertHrCommand,
  wrapInBulletListCommand,
  wrapInOrderedListCommand,
  toggleStrongCommand,
  toggleEmphasisCommand,
  toggleInlineCodeCommand,
} from '@milkdown/kit/preset/commonmark';
import { toggleStrikethroughCommand, insertTableCommand } from '@milkdown/kit/preset/gfm';
import { mount, unmount } from 'svelte';
import SlashMenu from './SlashMenu.svelte';

/**
 * @typedef {Object} SlashCommandItem
 * @property {string} id
 * @property {string} label
 * @property {string} description
 * @property {string[]} keywords
 * @property {(ctx: import('@milkdown/kit/ctx').Ctx) => void} run
 */

/** @type {SlashCommandItem[]} */
const items = [
  {
    id: 'h1',
    label: 'Heading 1',
    description: 'Big section heading',
    keywords: ['h1', 'heading', 'title'],
    run: (ctx) => ctx.get(commandsCtx).call(wrapInHeadingCommand.key, 1),
  },
  {
    id: 'h2',
    label: 'Heading 2',
    description: 'Medium section heading',
    keywords: ['h2', 'heading', 'subtitle'],
    run: (ctx) => ctx.get(commandsCtx).call(wrapInHeadingCommand.key, 2),
  },
  {
    id: 'h3',
    label: 'Heading 3',
    description: 'Small section heading',
    keywords: ['h3', 'heading'],
    run: (ctx) => ctx.get(commandsCtx).call(wrapInHeadingCommand.key, 3),
  },
  {
    id: 'bold',
    label: 'Bold',
    description: 'Toggle bold at cursor',
    keywords: ['bold', 'strong'],
    run: (ctx) => ctx.get(commandsCtx).call(toggleStrongCommand.key),
  },
  {
    id: 'italic',
    label: 'Italic',
    description: 'Toggle italic at cursor',
    keywords: ['italic', 'emphasis', 'em'],
    run: (ctx) => ctx.get(commandsCtx).call(toggleEmphasisCommand.key),
  },
  {
    id: 'strikethrough',
    label: 'Strikethrough',
    description: 'Toggle strikethrough at cursor',
    keywords: ['strikethrough', 'strike', 'del'],
    run: (ctx) => ctx.get(commandsCtx).call(toggleStrikethroughCommand.key),
  },
  {
    id: 'inline-code',
    label: 'Inline Code',
    description: 'Toggle inline code at cursor',
    keywords: ['code', 'inline'],
    run: (ctx) => ctx.get(commandsCtx).call(toggleInlineCodeCommand.key),
  },
  {
    id: 'bullet-list',
    label: 'Bullet List',
    description: 'Unordered list',
    keywords: ['bullet', 'list', 'ul', 'unordered'],
    run: (ctx) => ctx.get(commandsCtx).call(wrapInBulletListCommand.key),
  },
  {
    id: 'ordered-list',
    label: 'Numbered List',
    description: 'Ordered list',
    keywords: ['number', 'ordered', 'list', 'ol'],
    run: (ctx) => ctx.get(commandsCtx).call(wrapInOrderedListCommand.key),
  },
  {
    id: 'blockquote',
    label: 'Quote',
    description: 'Blockquote',
    keywords: ['quote', 'blockquote', 'citation'],
    run: (ctx) => ctx.get(commandsCtx).call(wrapInBlockquoteCommand.key),
  },
  {
    id: 'code-block',
    label: 'Code Block',
    description: 'Fenced code block',
    keywords: ['code', 'block', 'fence', 'snippet'],
    run: (ctx) => ctx.get(commandsCtx).call(createCodeBlockCommand.key),
  },
  {
    id: 'table',
    label: 'Table',
    description: 'Insert a 3x3 table',
    keywords: ['table', 'grid'],
    run: (ctx) => ctx.get(commandsCtx).call(insertTableCommand.key),
  },
  {
    id: 'divider',
    label: 'Divider',
    description: 'Horizontal rule',
    keywords: ['hr', 'divider', 'rule', 'line'],
    run: (ctx) => ctx.get(commandsCtx).call(insertHrCommand.key),
  },
];

/**
 * Subsequence fuzzy match with a lightweight score. Returns null when the
 * query's characters don't all appear in order within the target.
 * @param {string} query
 * @param {string} target
 * @returns {number | null}
 */
function fuzzyScore(query, target) {
  if (!query) return 0;

  const q = query.toLowerCase();
  const t = target.toLowerCase();

  let score = 0;
  let tIndex = 0;
  let consecutive = 0;

  for (let qIndex = 0; qIndex < q.length; qIndex++) {
    const char = q[qIndex];
    const foundAt = t.indexOf(char, tIndex);
    if (foundAt === -1) return null;

    consecutive = foundAt === tIndex ? consecutive + 1 : 0;
    score += foundAt === 0 ? 3 : consecutive > 0 ? 2 : 1;
    tIndex = foundAt + 1;
  }

  return score;
}

/**
 * @param {string} query
 * @returns {SlashCommandItem[]}
 */
function filterItems(query) {
  if (!query) return items;

  const scored = items
    .map((item) => {
      const best = Math.max(
        ...[item.label, ...item.keywords].map((field) => fuzzyScore(query, field) ?? -1)
      );
      return { item, best };
    })
    .filter(({ best }) => best >= 0);

  scored.sort((a, b) => b.best - a.best);
  return scored.map(({ item }) => item);
}

export const slashMenu = prose((ctx) => {
  const content = document.createElement('div');
  content.classList.add('slash-menu-portal');
  content.style.position = 'absolute';
  content.style.zIndex = '50';

  /** @type {{ query: string, filtered: SlashCommandItem[], selectedIndex: number }} */
  const menuState = $state({ query: '', filtered: items, selectedIndex: 0 });

  /** @param {SlashCommandItem} item */
  function selectItem(item) {
    const view = ctx.get(editorViewCtx);
    const { state, dispatch } = view;
    const { from } = state.selection;
    const start = from - (menuState.query.length + 1);

    dispatch(state.tr.delete(start, from));
    item.run(ctx);
    view.focus();
    provider.hide();
  }

  const component = mount(SlashMenu, {
    target: content,
    props: {
      get items() {
        return menuState.filtered;
      },
      get selectedIndex() {
        return menuState.selectedIndex;
      },
      onSelect: selectItem,
    },
  });

  const provider = new SlashProvider({
    content,
    trigger: '/',
    debounce: 20,
    shouldShow(view) {
      const currentText = provider.getContent(
        view,
        (node) => node.type.name === 'paragraph' || node.type.name === 'heading'
      );
      if (currentText == null || !currentText.startsWith('/')) return false;

      menuState.query = currentText.slice(1);
      menuState.filtered = filterItems(menuState.query);
      menuState.selectedIndex = 0;
      return menuState.filtered.length > 0;
    },
  });

  provider.onHide = () => {
    menuState.query = '';
  };

  return new Plugin({
    view: () => ({
      update: (view, prevState) => provider.update(view, prevState),
      destroy: () => {
        unmount(component);
        provider.destroy();
        content.remove();
      },
    }),
    props: {
      handleKeyDown(_view, event) {
        if (content.dataset.show !== 'true') return false;

        if (event.key === 'ArrowDown') {
          event.preventDefault();
          menuState.selectedIndex = (menuState.selectedIndex + 1) % menuState.filtered.length;
          return true;
        }
        if (event.key === 'ArrowUp') {
          event.preventDefault();
          menuState.selectedIndex =
            (menuState.selectedIndex - 1 + menuState.filtered.length) % menuState.filtered.length;
          return true;
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
          const target = menuState.filtered[menuState.selectedIndex];
          if (target) {
            event.preventDefault();
            selectItem(target);
            return true;
          }
        }
        if (event.key === 'Escape') {
          event.preventDefault();
          provider.hide();
          return true;
        }
        return false;
      },
    },
  });
});
