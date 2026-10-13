/**
 * @file Ranges drawn over the text that are not part of the document.
 *
 * Find highlights, codex mentions, grammar squiggles and (later) another
 * writer's cursor are all the same thing: a span of a block that should look
 * different without being *in* the block. Putting them in the document would
 * put them in the Markdown and in the undo history, which is not what any of
 * them means.
 *
 * ProseMirror calls these decorations and maps their positions through every
 * transaction. Nothing here maps anything, because nothing here needs to: a
 * find highlight and a mention are both *derived from the block's text*, so
 * when a block changes the honest answer is to scan it again rather than to
 * guess where its highlights slid to. The view already redraws per block, and
 * {@link decorate} is cheap enough to run in that redraw.
 *
 * Offsets are UTF-16 units, the same as everywhere else the view counts.
 */

/**
 * @typedef {Object} Decoration
 * @property {number} block
 * @property {number} start
 * @property {number} end - Exclusive. A zero-width range draws nothing.
 * @property {string} class - A `deco-*` class; `rust-editor.svelte` styles the
 *   ones it knows and ignores the rest.
 * @property {Record<string, string>} [attrs] - Put on the element as written: a
 *   `title` for a hover, a `data-*` for the caller to hook on to. Not for
 *   anything that changes how the text measures — `dom.js` counts text nodes
 *   and `data-atom`, and a decoration must not add either.
 */

/** @typedef {import('$lib/rust-editor.js').EditorRun} EditorRun */
/** @typedef {import('$lib/rust-editor.js').EditorBlock} EditorBlock */

/**
 * A block's text as the view counts it, so a match's offsets line up with the
 * engine's. An atom is one unit of *something*: ￼ is the Unicode
 * object-replacement character, the same stand-in `dom.js` uses.
 * @param {EditorRun[]} runs
 * @returns {string}
 */
export function textOfRuns(runs) {
  return (runs ?? []).map((run) => (run.type === 'text' ? (run.text ?? '') : '￼')).join('');
}

/**
 * Every block in a tree, in document order, nested ones included — the order a
 * find walks them in.
 * @param {EditorBlock[]} blocks
 * @returns {EditorBlock[]}
 */
export function flattenBlocks(blocks) {
  /** @type {EditorBlock[]} */
  const out = [];
  for (const block of blocks ?? []) {
    out.push(block);
    const nested = [
      ...(block.children ?? []),
      ...(block.items ?? []).flatMap((item) => item.blocks ?? []),
      ...(block.rows ?? []).flatMap((row) => (row.cells ?? []).flat()),
    ];
    out.push(...flattenBlocks(nested));
  }
  return out;
}

/**
 * Decorations for every match of `pattern` in a document.
 *
 * The pattern is used with `lastIndex`, so it has to be global; a non-global
 * one would match the first occurrence in a block forever. A zero-length match
 * advances by one rather than looping.
 * @param {EditorBlock[]} blocks
 * @param {RegExp} pattern - Global.
 * @param {string} className
 * @returns {Decoration[]}
 */
export function matchDecorations(blocks, pattern, className) {
  /** @type {Decoration[]} */
  const out = [];
  for (const block of flattenBlocks(blocks)) {
    const text = block.runs ? textOfRuns(block.runs) : (block.text ?? '');
    if (!text) continue;
    pattern.lastIndex = 0;
    let match;
    while ((match = pattern.exec(text)) !== null) {
      if (match[0].length > 0) {
        out.push({
          block: block.id,
          start: match.index,
          end: match.index + match[0].length,
          class: className,
        });
      } else {
        pattern.lastIndex += 1;
      }
    }
  }
  return out;
}

/**
 * Splits a block's runs so that every decoration boundary falls between two
 * runs, and hangs the decorations' classes on the pieces they cover.
 *
 * The run list the engine sent is not changed: a new one comes back, with
 * `length` and every other property of each piece preserved, so the DOM the
 * view builds still measures the way `dom.js` expects. An atom cannot be
 * split, so a decoration touching one covers all of it.
 * @param {EditorRun[]} runs
 * @param {Decoration[]} decorations - For this block only.
 * @returns {(EditorRun & { deco?: string, decoAttrs?: Record<string, string> })[]}
 */
export function decorate(runs, decorations) {
  const list = runs ?? [];
  const ranges = (decorations ?? []).filter((d) => d.end > d.start);
  if (ranges.length === 0) return list;

  // Every offset a decoration starts or ends at is a place a run has to break.
  const cuts = new Set();
  for (const range of ranges) {
    cuts.add(range.start);
    cuts.add(range.end);
  }

  /** @type {(EditorRun & { deco?: string })[]} */
  const out = [];
  let at = 0;
  for (const run of list) {
    const length = run.length ?? (run.type === 'text' ? (run.text ?? '').length : 1);
    const end = at + length;

    // Only a text run can be cut; an atom is one indivisible unit.
    if (run.type !== 'text') {
      out.push(withClasses(run, ranges, at, end));
      at = end;
      continue;
    }

    const breaks = [...cuts].filter((cut) => cut > at && cut < end).sort((a, b) => a - b);
    let from = at;
    for (const cut of [...breaks, end]) {
      const text = (run.text ?? '').slice(from - at, cut - at);
      out.push(withClasses({ ...run, text, length: text.length }, ranges, from, cut));
      from = cut;
    }
    at = end;
  }
  return out;
}

/**
 * The run with the classes of every decoration that covers it, or the run
 * unchanged when none does.
 * @param {EditorRun} run
 * @param {Decoration[]} ranges
 * @param {number} start
 * @param {number} end
 * @returns {EditorRun & { deco?: string, decoAttrs?: Record<string, string> }}
 */
function withClasses(run, ranges, start, end) {
  const covering = ranges.filter((range) => range.start < end && range.end > start);
  if (covering.length === 0) return run;
  const classes = [...new Set(covering.map((range) => range.class))].join(' ');
  // Later decorations win a contested attribute, which is the order the caller
  // put them in.
  const attrs = Object.assign({}, ...covering.map((range) => range.attrs ?? {}));
  const decorated = { ...run, deco: classes };
  if (Object.keys(attrs).length > 0) decorated.decoAttrs = attrs;
  return decorated;
}

/**
 * The decorations for one block, which is all its renderer needs.
 * @param {Decoration[]} decorations
 * @param {number} block
 * @returns {Decoration[]}
 */
export function forBlock(decorations, block) {
  return (decorations ?? []).filter((decoration) => decoration.block === block);
}
