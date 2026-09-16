/**
 * @file Scene metadata kept as YAML front matter at the top of a writing:
 *
 *   ---
 *   synopsis: "She finds the letter."
 *   status: "First Draft"
 *   ---
 *
 * Only the keys below are read and written. Anything else in the block (tags
 * from Obsidian, say) is kept exactly as it was. Values are written as JSON
 * strings, which are valid YAML, so quotes and new lines survive.
 */

/** Keys the app understands, in the order they are written. */
export const META_KEYS = /** @type {const} */ (['synopsis', 'status', 'label']);

/** Scrivener's default statuses, offered as presets. Any text is allowed. */
export const STATUS_PRESETS = ['To Do', 'First Draft', 'Revised Draft', 'Final Draft', 'Done'];

/**
 * @typedef {{ synopsis?: string, status?: string, label?: string }} SceneMeta
 * @typedef {{ meta: SceneMeta, extra: string[], body: string, raw: string }} ParsedWriting
 *   `raw` is the front matter exactly as it was in the file ('' if none).
 */

const BLOCK_RE = /^---\r?\n([\s\S]*?)\r?\n---[ \t]*(?:\r?\n|$)/;

/** @param {string} raw */
function parseScalar(raw) {
  const value = raw.trim();
  if (value.startsWith('"')) {
    try {
      return JSON.parse(value);
    } catch {
      return value.slice(1, value.endsWith('"') ? -1 : undefined);
    }
  }
  if (value.startsWith("'") && value.endsWith("'") && value.length >= 2) {
    return value.slice(1, -1).replaceAll("''", "'");
  }
  return value;
}

/**
 * Splits a writing into its metadata and body.
 * @param {string} markdown
 * @returns {ParsedWriting}
 */
export function parseWriting(markdown) {
  const text = markdown ?? '';
  const match = BLOCK_RE.exec(text);
  // A document can also open with a horizontal rule; only a block that reads
  // like YAML (key: value lines, indented or list continuations) is metadata.
  const looksLikeYaml =
    match &&
    /^[A-Za-z_][\w-]*:/.test(match[1]) &&
    match[1].split(/\r?\n/).every((line) => line === '' || /^([A-Za-z_][\w-]*:|\s|- |#)/.test(line));
  if (!match || !looksLikeYaml) return { meta: {}, extra: [], body: text, raw: '' };

  /** @type {SceneMeta} */
  const meta = {};
  /** @type {string[]} */
  const extra = [];
  const lines = match[1].split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // An entry is its key line plus any indented lines that continue it.
    let end = i + 1;
    while (end < lines.length && (/^\s/.test(lines[end]) || (lines[end] === '' && /^\s/.test(lines[end + 1] ?? '')))) {
      end++;
    }
    const continuation = lines.slice(i + 1, end);
    const keyMatch = /^([A-Za-z_][\w-]*):(.*)$/.exec(line);
    const key = /** @type {keyof SceneMeta | undefined} */ (keyMatch?.[1]);
    const value = keyMatch?.[2].trim() ?? '';
    const ours = key && META_KEYS.includes(/** @type {any} */ (key)) && !(key in meta);

    if (ours && /^[|>][+-]?$/.test(value)) {
      // Block scalar: `|` keeps line breaks, `>` folds them into spaces.
      const indent = Math.min(...continuation.filter((l) => l.trim()).map((l) => l.match(/^\s*/)[0].length));
      const body = continuation.map((l) => l.slice(Number.isFinite(indent) ? indent : 0));
      meta[key] = (value.startsWith('|') ? body.join('\n') : body.join(' ').replace(/\s+/g, ' ')).trim();
    } else if (ours && value !== '') {
      // Plain scalars may wrap onto indented lines; YAML joins them with spaces.
      const folded = [value, ...continuation.map((l) => l.trim())].filter(Boolean).join(' ');
      meta[key] = String(parseScalar(folded));
    } else {
      extra.push(line, ...continuation);
    }
    i = end - 1;
  }
  // A blank line conventionally follows the block; it belongs to the block.
  const blank = /^\r?\n/.exec(text.slice(match[0].length))?.[0] ?? '';
  const raw = text.slice(0, match[0].length + blank.length);
  return { meta, extra, body: text.slice(raw.length), raw };
}

/** @param {SceneMeta} a @param {SceneMeta} b */
function sameMeta(a, b) {
  return META_KEYS.every((key) => (a[key]?.trim() ?? '') === (b[key]?.trim() ?? ''));
}

/**
 * Joins metadata and body back into a file. Without metadata or extra lines
 * there is no front matter at all. When `original` is given and the metadata
 * hasn't changed, its block is written back byte for byte, so saving the text
 * never reformats someone's YAML.
 * @param {SceneMeta} meta
 * @param {string[]} extra - Unrecognised front matter lines, kept verbatim.
 * @param {string} body
 * @param {{ meta: SceneMeta, raw: string }} [original]
 * @returns {string}
 */
export function serializeWriting(meta, extra, body, original) {
  if (original && sameMeta(meta, original.meta)) return original.raw + body;
  const lines = [];
  for (const key of META_KEYS) {
    const value = meta[key]?.trim();
    if (value) lines.push(`${key}: ${JSON.stringify(value)}`);
  }
  const kept = extra.filter((line, i) => line.trim() !== '' || i < extra.length - 1);
  lines.push(...kept);
  if (lines.length === 0) return body;
  return `---\n${lines.join('\n')}\n---\n\n${body}`;
}
