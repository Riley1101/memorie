/** Custom MIME type so the tree only reacts to its own drags, not stray files. */
export const DRAG_MIME = 'application/x-memoire-tree';

/**
 * @typedef {{ kind: 'file', name: string, label: string } | { kind: 'folder', path: string, label: string }} DragPayload
 */

/**
 * @param {DragEvent} e
 * @returns {DragPayload | null}
 */
export function readDragPayload(e) {
  try {
    const raw = e.dataTransfer?.getData(DRAG_MIME);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (parsed?.kind === 'file' && typeof parsed.name === 'string') return parsed;
    if (parsed?.kind === 'folder' && typeof parsed.path === 'string') return parsed;
    return null;
  } catch {
    return null;
  }
}
