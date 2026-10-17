import { fileManager, dirOf, baseOf } from '$lib/runes/fs.svelte.js';
import { sceneCache } from '$lib/runes/scenes.svelte.js';
import { serializeWriting } from '$lib/front-matter.js';
import { toast } from '$lib/toast.js';

/**
 * @file Manual order for writings inside a folder.
 *
 * A novel's scenes have an order, and it is rarely alphabetical. Without this
 * the only way to sequence a manuscript is to number every filename, which
 * then fights the title that follows the first line. The order lives in each
 * writing's own front matter, so it travels with the file, survives a sync and
 * stays readable to anything else that opens the folder.
 *
 * Keys are strings so they sort lexicographically after zero-padding, and so a
 * writing with no key at all still sorts sensibly (after the ordered ones, by
 * name).
 */

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

/** Width the index is padded to: room for far more scenes than anyone writes. */
const WIDTH = 4;
/** Gap between neighbours, so a single insert rarely needs a renumber. */
const STEP = 10;

/** @param {number} n */
export function orderKey(n) {
  return String(Math.max(0, Math.round(n))).padStart(WIDTH, '0');
}

/**
 * Sort position of a writing: its key, or `null` when it has none.
 * @param {string} name
 * @returns {string | null}
 */
export function orderOf(name) {
  const raw = sceneCache.get(name)?.meta.order?.trim();
  return raw ? raw.padStart(WIDTH, '0') : null;
}

/**
 * Compares two writings by manual order, falling back to the caller's
 * comparator for anything unordered. Ordered writings always come first, so
 * sequencing part of a folder does something sensible.
 * @param {string} aName
 * @param {string} bName
 * @param {(a: string, b: string) => number} fallback
 */
export function compareByOrder(aName, bName, fallback) {
  const a = orderOf(aName);
  const b = orderOf(bName);
  if (a !== null && b !== null) return a === b ? fallback(aName, bName) : a < b ? -1 : 1;
  if (a !== null) return -1;
  if (b !== null) return 1;
  return fallback(aName, bName);
}

/** True when anything in `dir` carries a manual order. @param {string} dir */
export function dirIsOrdered(dir) {
  return fileManager.files.some((f) => dirOf(f.name) === dir && orderOf(f.name) !== null);
}

/**
 * Writes an order key into a writing's front matter, leaving its text alone.
 * @param {string} name
 * @param {string} key
 */
async function writeOrder(name, key) {
  const scene = sceneCache.get(name);
  if (!scene) throw new Error(`"${baseOf(name)}" has not been read yet`);
  if ((scene.meta.order ?? '') === key) return;
  const meta = { ...scene.meta, order: key };
  await fileManager.saveFile(
    name,
    serializeWriting(meta, scene.extra, scene.body, { meta: scene.meta, raw: scene.raw })
  );
  // Show it at once; the refreshed listing re-reads the file anyway.
  sceneCache.entries.set(name, { ...scene, meta });
}

/**
 * Moves `moved` so that it sits where `target` is, and renumbers the folder
 * from the result. Renumbering the whole folder keeps the keys tidy and makes
 * the operation idempotent, at the cost of rewriting each file's front matter
 * once — which is cheap next to the text already being saved on every pause.
 *
 * @param {string} dir - Folder both writings live in ('' for top level).
 * @param {string} moved - Content-relative name being dragged.
 * @param {string} target - The writing it was dropped onto.
 * @param {'before' | 'after'} side
 * @returns {Promise<boolean>}
 */
export async function reorderWithin(dir, moved, target, side) {
  const inDir = fileManager.files.filter((f) => dirOf(f.name) === dir).map((f) => f.name);
  if (!inDir.includes(moved) || !inDir.includes(target) || moved === target) return false;

  // Everything has to be read before its front matter can be rewritten.
  await sceneCache.load(fileManager.files.filter((f) => dirOf(f.name) === dir));

  const current = currentOrder(dir);
  const without = current.filter((name) => name !== moved);
  const at = without.indexOf(target);
  if (at === -1) return false;
  without.splice(side === 'before' ? at : at + 1, 0, moved);

  try {
    for (const [i, name] of without.entries()) {
      await writeOrder(name, orderKey((i + 1) * STEP));
    }
    await fileManager.getRecents();
    return true;
  } catch (e) {
    console.error('Could not reorder:', e);
    toast.error('Could not change the order', e);
    return false;
  }
}

/**
 * The writings directly inside `dir`, in the order they are shown.
 * @param {string} dir
 * @returns {string[]}
 */
export function currentOrder(dir) {
  return fileManager.files
    .filter((f) => dirOf(f.name) === dir)
    .map((f) => f.name)
    .sort((a, b) => compareByOrder(a, b, (x, y) => collator.compare(baseOf(x), baseOf(y))));
}

/**
 * Drops the manual order from every writing in `dir`, so it goes back to
 * sorting by name.
 * @param {string} dir
 */
export async function clearOrder(dir) {
  const files = fileManager.files.filter((f) => dirOf(f.name) === dir);
  await sceneCache.load(files);
  try {
    for (const file of files) {
      const scene = sceneCache.get(file.name);
      if (!scene?.meta.order) continue;
      const meta = { ...scene.meta, order: undefined };
      await fileManager.saveFile(
        file.name,
        serializeWriting(meta, scene.extra, scene.body, { meta: scene.meta, raw: scene.raw })
      );
      sceneCache.entries.set(file.name, { ...scene, meta });
    }
    await fileManager.getRecents();
    toast.success('Back to sorting by name');
    return true;
  } catch (e) {
    toast.error('Could not clear the order', e);
    return false;
  }
}
