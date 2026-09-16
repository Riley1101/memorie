import { $prose as prose } from '@milkdown/kit/utils';
import { editorViewCtx } from '@milkdown/kit/core';
import { SlashProvider } from '@milkdown/kit/plugin/slash';
import { Plugin } from '@milkdown/kit/prose/state';
import { mount, unmount } from 'svelte';
import DocRefMenu from './DocRefMenu.svelte';
import { fuzzyScore } from './fuzzy.js';
import { fileManager, dirOf, baseOf } from '$lib/runes/fs.svelte.js';
import { invoke } from '@tauri-apps/api/core';

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

/**
 * @typedef {Object} Backlink
 * @property {string} name - File that references the target.
 * @property {string} title - Display title of that file.
 * @property {string} folder - Parent folder, or "" for top-level.
 * @property {string} snippet - The line holding the first reference, links flattened to their text.
 */

function escapeRegExp(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Contents of writings, keyed by name and valid while `modified` matches the
 * file list. Writings without any doc reference store `null` so they're never
 * read again until they change.
 * @type {Map<string, { modified: number, content: string | null }>}
 */
const contentCache = new Map();

/**
 * Reads the writings whose cached copy is missing or out of date.
 * @param {{ name: string, last_modified: number }[]} files
 */
async function refreshContentCache(files) {
  const stale = files.filter((f) => contentCache.get(f.name)?.modified !== f.last_modified);
  if (stale.length > 0) {
    /** @type {[string, { Ok?: string, Err?: string } | string][]} */
    const results = await invoke('read_files', { names: stale.map((f) => f.name) });
    const modified = new Map(stale.map((f) => [f.name, f.last_modified]));
    for (const [name, result] of results) {
      const content = typeof result === 'string' ? result : result?.Ok;
      if (typeof content !== 'string') continue;
      contentCache.set(name, {
        modified: modified.get(name) ?? 0,
        content: content.includes(DOC_REF_PREFIX) ? content : null,
      });
    }
  }
  // Drop writings that were deleted or renamed away.
  if (contentCache.size > files.length) {
    const live = new Set(files.map((f) => f.name));
    for (const name of contentCache.keys()) if (!live.has(name)) contentCache.delete(name);
  }
}

/**
 * Finds every writing that links to `target` via a `[[` doc reference.
 * @param {string} target - Content-relative file name.
 * @returns {Promise<Backlink[]>}
 */
export async function findBacklinks(target) {
  const others = fileManager.files.filter((f) => f.name !== target);
  if (others.length === 0) return [];
  await refreshContentCache(others);

  // Milkdown writes the encoded href, but accept a hand-typed raw path too.
  const hrefs = [...new Set([encodeURIComponent(target), target])].map(
    (h) => `${DOC_REF_PREFIX}${h}`
  );
  const refRe = new RegExp(`\\]\\((?:${hrefs.map(escapeRegExp).join('|')})\\)`);

  /** @type {Backlink[]} */
  const backlinks = [];
  for (const [name, { content }] of contentCache) {
    if (name === target || content === null) continue;
    if (!hrefs.some((h) => content.includes(`](${h})`))) continue;
    const line = content.split('\n').find((l) => refRe.test(l));
    if (line === undefined) continue;
    backlinks.push({
      name,
      title: stripMd(baseOf(name)),
      folder: dirOf(name),
      snippet: line
        .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
        .replace(/^\s*(#{1,6}\s+|[-*+]\s+|\d+\.\s+|>\s*)/, '')
        .replace(/[*_`~]/g, '')
        .trim(),
    });
  }
  return backlinks.sort((a, b) => a.title.localeCompare(b.title));
}
