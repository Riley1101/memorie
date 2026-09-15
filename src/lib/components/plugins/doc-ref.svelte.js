import { $prose as prose } from '@milkdown/kit/utils';
import { editorViewCtx } from '@milkdown/kit/core';
import { SlashProvider } from '@milkdown/kit/plugin/slash';
import { Plugin } from '@milkdown/kit/prose/state';
import { mount, unmount } from 'svelte';
import DocRefMenu from './DocRefMenu.svelte';
import { fuzzyScore } from './fuzzy.js';
import { fileManager, dirOf, baseOf } from '$lib/runes/fs.svelte.js';

/**
 * Prefix marking a link's href as an internal doc reference rather than an
 * external URL. Deliberately scheme-less (no "protocol:") — Milkdown's link
 * mark strips any href with an unrecognized scheme when rendering to the DOM.
 */
export const DOC_REF_PREFIX = '#writing/';

/**
 * @typedef {Object} DocRefItem
 * @property {string} name - Content-relative file name, e.g. "Novel/Ch 1.md".
 * @property {string} title - Display title, no folder, no ".md".
 * @property {string} folder - Parent folder, or "" for top-level.
 */

function stripMd(name) {
  return name.replace(/\.md$/i, '').trim();
}

/** @returns {DocRefItem[]} */
function allItems() {
  return fileManager.files.map((f) => ({
    name: f.name,
    title: stripMd(baseOf(f.name)),
    folder: dirOf(f.name),
  }));
}

/**
 * @param {string} query
 * @returns {DocRefItem[]}
 */
function filterItems(query) {
  const items = allItems();
  if (!query) return items.slice(0, 20);

  const scored = items
    .map((item) => ({ item, score: fuzzyScore(query, item.title) }))
    .filter(({ score }) => score !== null);

  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, 20).map(({ item }) => item);
}

/** Matches an unclosed `[[query` at the end of the current text block. */
const TRIGGER_RE = /\[\[([^[\]]*)$/;

export const docRefMenu = prose((ctx) => {
  const content = document.createElement('div');
  content.classList.add('doc-ref-portal');
  content.style.position = 'absolute';
  content.style.zIndex = '50';

  /** @type {{ query: string, filtered: DocRefItem[], selectedIndex: number }} */
  const menuState = $state({ query: '', filtered: [], selectedIndex: 0 });

  /** @param {DocRefItem} item */
  function selectItem(item) {
    const view = ctx.get(editorViewCtx);
    const { state, dispatch } = view;
    const { from } = state.selection;
    const start = from - (menuState.query.length + 2); // "[[" + query

    const linkMark = state.schema.marks.link.create({
      href: `${DOC_REF_PREFIX}${encodeURIComponent(item.name)}`,
    });
    const node = state.schema.text(item.title || item.name, [linkMark]);

    dispatch(state.tr.delete(start, from).insert(start, node));
    view.focus();
    provider.hide();
  }

  const component = mount(DocRefMenu, {
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
    trigger: '[[',
    debounce: 20,
    shouldShow(view) {
      const currentText = provider.getContent(
        view,
        (node) => node.type.name === 'paragraph' || node.type.name === 'heading'
      );
      if (currentText == null) return false;

      const match = TRIGGER_RE.exec(currentText);
      if (!match) return false;

      menuState.query = match[1];
      menuState.filtered = filterItems(menuState.query);
      menuState.selectedIndex = 0;
      return true;
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
          if (menuState.filtered.length > 0) {
            menuState.selectedIndex = (menuState.selectedIndex + 1) % menuState.filtered.length;
          }
          return true;
        }
        if (event.key === 'ArrowUp') {
          event.preventDefault();
          if (menuState.filtered.length > 0) {
            menuState.selectedIndex =
              (menuState.selectedIndex - 1 + menuState.filtered.length) % menuState.filtered.length;
          }
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
