import { buildFileTree, formatFileName } from '$lib/utils.js';
import { baseOf } from '$lib/runes/fs.svelte.js';

/**
 * @typedef {{ kind: 'heading', level: number, text: string }
 *   | { kind: 'document', name: string, title: string | null }} ExportItem
 * @typedef {{ kind: 'folder' | 'file', path: string, label: string, depth: number }} OutlineEntry
 *   `path` is content-relative: "Novel/Part One" or "Novel/Part One/Scene.md".
 */

/**
 * Everything under a folder, in the same order the binder tree shows it.
 *
 * @param {{ name: string, folder: string | null }[]} files
 * @param {string[]} folders
 * @param {string} dir - A binder or any folder inside one.
 * @returns {OutlineEntry[]}
 */
export function manuscriptOutline(files, folders, dir) {
  const [binder, ...inner] = dir.split('/');
  let nodes = buildFileTree(files, binder, folders);
  for (const segment of inner) {
    const folder = nodes.find((n) => n.type === 'folder' && n.name === segment);
    if (!folder) return [];
    nodes = folder.children;
  }

  /** @type {OutlineEntry[]} */
  const entries = [];
  const walk = (/** @type {any[]} */ level, /** @type {number} */ depth) => {
    // The tree lists folders first; a manuscript reads better with a folder's
    // own writings (an intro, a prologue) before its sub-folders.
    const ordered = [
      ...level.filter((n) => n.type === 'file'),
      ...level.filter((n) => n.type === 'folder'),
    ];
    for (const node of ordered) {
      if (node.type === 'folder') {
        entries.push({ kind: 'folder', path: [binder, ...node.path].join('/'), label: node.name, depth });
        walk(node.children, depth + 1);
      } else {
        entries.push({ kind: 'file', path: node.file.name, label: formatFileName(baseOf(node.file.name)), depth });
      }
    }
  };
  walk(nodes, 1);
  return entries;
}

/**
 * The manuscript for a folder: each folder becomes a heading (level by depth),
 * each writing a document. Anything in `excluded`, or inside an excluded
 * folder, is left out, and so are folders with no writings left.
 *
 * @param {{ name: string, folder: string | null }[]} files
 * @param {string[]} folders
 * @param {string} dir - A binder or any folder inside one.
 * @param {{ writingTitles: boolean, excluded?: Set<string> }} options
 * @returns {ExportItem[]}
 */
export function manuscriptItems(files, folders, dir, { writingTitles, excluded = new Set() }) {
  const kept = manuscriptOutline(files, folders, dir).filter(
    (entry) => !isExcluded(entry.path, excluded)
  );
  /** @type {ExportItem[]} */
  const items = [];
  kept.forEach((entry, i) => {
    if (entry.kind === 'file') {
      items.push({
        kind: 'document',
        name: entry.path,
        title: writingTitles ? entry.label : null,
      });
      return;
    }
    // A folder is worth a heading only if a writing follows inside it.
    const inside = kept.slice(i + 1).findIndex((e) => e.depth <= entry.depth);
    const section = kept.slice(i + 1, inside === -1 ? undefined : i + 1 + inside);
    if (section.some((e) => e.kind === 'file')) {
      items.push({ kind: 'heading', level: Math.min(entry.depth, 6), text: entry.label });
    }
  });
  return items;
}

/**
 * Whether `path` or any folder above it is in `excluded`.
 * @param {string} path
 * @param {Set<string>} excluded
 */
export function isExcluded(path, excluded) {
  if (excluded.size === 0) return false;
  for (let p = path; p; p = p.includes('/') ? p.slice(0, p.lastIndexOf('/')) : '') {
    if (excluded.has(p)) return true;
  }
  return false;
}
