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
import { toggleStrikethroughCommand } from '@milkdown/kit/preset/gfm';
import { mount, unmount } from 'svelte';
import SlashMenu from './SlashMenu.svelte';
import { editLink } from './link-popover.svelte.js';
import { MOD_KEY } from '$lib/keyboard.svelte.js';
import { slashItems, filterSlashItems } from '$lib/editor/slash-items.js';

/**
 * What each shared item does in Milkdown, by id.
 *
 * The list itself — labels, keywords, order — lives in
 * `$lib/editor/slash-items.js` in the engine's vocabulary, because that is the
 * one that survives. Only the doing is here, so the two menus cannot come to
 * offer different things.
 * @type {Record<string, (ctx: import('@milkdown/kit/ctx').Ctx) => void>}
 */
const runners = {
  h1: (ctx) => ctx.get(commandsCtx).call(wrapInHeadingCommand.key, 1),
  h2: (ctx) => ctx.get(commandsCtx).call(wrapInHeadingCommand.key, 2),
  h3: (ctx) => ctx.get(commandsCtx).call(wrapInHeadingCommand.key, 3),
  bold: (ctx) => ctx.get(commandsCtx).call(toggleStrongCommand.key),
  italic: (ctx) => ctx.get(commandsCtx).call(toggleEmphasisCommand.key),
  strikethrough: (ctx) => ctx.get(commandsCtx).call(toggleStrikethroughCommand.key),
  'inline-code': (ctx) => ctx.get(commandsCtx).call(toggleInlineCodeCommand.key),
  'bullet-list': (ctx) => ctx.get(commandsCtx).call(wrapInBulletListCommand.key),
  'ordered-list': (ctx) => ctx.get(commandsCtx).call(wrapInOrderedListCommand.key),
  blockquote: (ctx) => ctx.get(commandsCtx).call(wrapInBlockquoteCommand.key),
  'code-block': (ctx) => ctx.get(commandsCtx).call(createCodeBlockCommand.key),
  link: (ctx) => editLink(ctx.get(editorViewCtx)),
  divider: (ctx) => ctx.get(commandsCtx).call(insertHrCommand.key),
};

const items = slashItems(MOD_KEY);

/**
 * @param {string} query
 * @returns {import('$lib/editor/slash-items.js').SlashItem[]}
 */
function filterItems(query) {
  return filterSlashItems(items, query);
}

export const slashMenu = prose((ctx) => {
  const content = document.createElement('div');
  content.classList.add('slash-menu-portal');
  content.style.position = 'absolute';
  content.style.zIndex = '50';

  /** @type {{ query: string, filtered: import('$lib/editor/slash-items.js').SlashItem[], selectedIndex: number }} */
  const menuState = $state({ query: '', filtered: items, selectedIndex: 0 });

  /** @param {import('$lib/editor/slash-items.js').SlashItem} item */
  function selectItem(item) {
    const view = ctx.get(editorViewCtx);
    const { state, dispatch } = view;
    const { from } = state.selection;
    const start = from - (menuState.query.length + 1);

    dispatch(state.tr.delete(start, from));
    runners[item.id]?.(ctx);
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
