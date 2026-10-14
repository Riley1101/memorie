/**
 * @file Content-relative path helpers.
 *
 * Pure string work, kept out of `runes/fs.svelte.js` so that a module needing
 * only these does not pull in the file manager — and with it the toast stack
 * and the Svelte runtime. `fs.svelte.js` re-exports them, so nothing that
 * already imports them from there has to change.
 */

/**
 * Directory part of a content-relative name, e.g. "Novel/Ch 1/Scene.md" -> "Novel/Ch 1".
 * @param {string} name
 * @returns {string}
 */
export function dirOf(name) {
  const i = name.lastIndexOf('/');
  return i === -1 ? '' : name.slice(0, i);
}

/**
 * Basename of a content-relative name, e.g. "Novel/Ch 1/Scene.md" -> "Scene.md".
 * @param {string} name
 * @returns {string}
 */
export function baseOf(name) {
  return name.slice(name.lastIndexOf('/') + 1);
}
