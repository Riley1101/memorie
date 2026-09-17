import { buildFileTree, formatFileName } from '$lib/utils.js';
import { baseOf } from '$lib/runes/fs.svelte.js';

/**
 * @typedef {{ kind: 'heading', level: number, text: string }
 *   | { kind: 'document', name: string, title: string | null }} ExportItem
 */

/**
 * The manuscript for a folder, in the same order the binder tree shows it:
 * each folder becomes a heading (level by depth), each writing a document.
 *
 * @param {{ name: string, folder: string | null }[]} files
 * @param {string[]} folders
 * @param {string} dir - A binder or any folder inside one.
 * @param {{ writingTitles: boolean }} options
 * @returns {ExportItem[]}
 */
export function manuscriptItems(files, folders, dir, { writingTitles }) {
  const [binder, ...inner] = dir.split('/');
  let nodes = buildFileTree(files, binder, folders);
  for (const segment of inner) {
    const folder = nodes.find((n) => n.type === 'folder' && n.name === segment);
    if (!folder) return [];
    nodes = folder.children;
  }

  /** @type {ExportItem[]} */
  const items = [];
  const walk = (/** @type {any[]} */ level, /** @type {number} */ depth) => {
    // The tree lists folders first; a manuscript reads better with a folder's
    // own writings (an intro, a prologue) before its sub-folders.
    const ordered = [
      ...level.filter((n) => n.type === 'file'),
      ...level.filter((n) => n.type === 'folder'),
    ];
    for (const node of ordered) {
      if (node.type === 'folder') {
        items.push({ kind: 'heading', level: Math.min(depth, 6), text: node.name });
        walk(node.children, depth + 1);
      } else {
        items.push({
          kind: 'document',
          name: node.file.name,
          title: writingTitles ? formatFileName(baseOf(node.file.name)) : null,
        });
      }
    }
  };
  walk(nodes, 1);
  return items;
}
