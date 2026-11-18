import { goto } from '$app/navigation';
import { resolve } from '$app/paths';

export const UNTITLED = 'Untitled';

/**
 * Characters that can't appear in a file name on any platform we ship to.
 * Slashes are excluded too so a title can never escape its folder.
 */
// eslint-disable-next-line no-control-regex
const UNSAFE = /[\\/:*?"<>|\x00-\x1f]/g;

/**
 * Turns free text into something safe to use as a file basename (no extension).
 * @param {string} text
 * @param {number} [maxLength=80]
 * @returns {string} Empty string if nothing usable is left.
 */
export function sanitizeTitle(text, maxLength = 80) {
  return (text ?? '')
    .replace(UNSAFE, '')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/\.+$/, '')
    .slice(0, maxLength)
    .trim();
}

/**
 * Strips inline markdown so a line reads as plain words.
 * @param {string} line
 */
function plainText(line) {
  return line
    .replace(/^[#>\-*+\d.)\s]+/, '') // heading/list/quote markers
    .replace(/!\[[^\]]*\]\([^)]*\)/g, '') // images
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1') // links -> text
    .replace(/[*_~`]+/g, '')
    .trim();
}

/**
 * Derives a title from a markdown body: the first heading wins, otherwise the
 * first handful of words from the first non-empty paragraph.
 * @param {string} markdown
 * @param {number} [maxWords=6]
 * @returns {string} Sanitized title, or empty string when the body is blank.
 */
export function deriveTitle(markdown, maxWords = 6) {
  const lines = (markdown ?? '').split('\n');

  const heading = lines.find((l) => /^#{1,6}\s+\S/.test(l));
  if (heading) {
    const text = sanitizeTitle(plainText(heading));
    if (text) return text;
  }

  for (const raw of lines) {
    if (/^\s*(```|---|\|)/.test(raw)) continue;
    const text = plainText(raw);
    if (!text) continue;
    const words = text.split(/\s+/);
    const short = words.slice(0, maxWords).join(' ');
    const title = sanitizeTitle(words.length > maxWords ? `${short}…` : short);
    if (title) return title;
  }

  return '';
}

/**
 * True when a basename is one of ours: "Untitled", "Untitled 2", ... (with or without .md).
 * @param {string} base
 */
export function isUntitled(base) {
  return new RegExp(`^${UNTITLED}( \\d+)?(\\.md)?$`).test((base ?? '').trim());
}

/**
 * Route for an unsaved draft that will live in `dir` once the writer types.
 * Nothing touches disk until then.
 * @param {string} [dir=''] Content-relative folder, e.g. "" or "Novel/Chapter 1".
 * @returns {string}
 */
export function draftUrl(dir = '') {
  const planned = dir ? `${dir}/${UNTITLED}.md` : `${UNTITLED}.md`;
  return writingUrl(planned);
}

/**
 * Route for an existing writing. The whole name, folders included, is one
 * encoded segment of the `[lexical]` route.
 * @param {string} name Content-relative file name, e.g. "Novel/Chapter 1.md".
 * @returns {string}
 */
export function writingUrl(name) {
  return resolve('/[lexical]', { lexical: encodeURIComponent(name) });
}

/**
 * Opens a blank editor in `dir`. The file is created on the first save.
 * @param {string} [dir='']
 */
export function startNewWriting(dir = '') {
  // The timestamp makes draft → draft in the same folder a real navigation
  // (fresh load, fresh editor) instead of a no-op on an identical URL.
  // eslint-disable-next-line svelte/no-navigation-without-resolve -- resolved path plus a query flag
  return goto(`${draftUrl(dir)}?new=${Date.now()}`);
}
