import { $prose } from '@milkdown/kit/utils';
import { Plugin, PluginKey } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';

/**
 * @file Underlines codex names and aliases in the editor, with the entry's
 * summary on hover. Matching follows `codex::Matcher` in the backend: whole
 * words only, each entry's case mode, and the longest name wins where names
 * overlap ("Iron Citadel" over "Citadel").
 *
 * The editor page hands the entries over with `setCodexMentions`; with none
 * (or the setting off) the plugin draws nothing.
 */

/** @typedef {import('$lib/runes/codex.svelte.js').Entity} Entity */

export const codexMentionsKey = new PluginKey('codex-mentions');

/** @param {string} s */
function escapeRegex(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * @typedef {{ regex: RegExp, owners: Map<string, Entity> }} Pattern
 * @typedef {{ patterns: Pattern[] }} Matcher
 */

/**
 * @param {Entity[]} entities
 * @returns {Matcher}
 */
export function buildMatcher(entities) {
  /** @type {Pattern[]} */
  const patterns = [];
  for (const insensitive of [false, true]) {
    /** @type {Map<string, Entity>} */
    const owners = new Map();
    for (const entity of entities) {
      if (entity.matchMode === 'off') continue;
      if ((entity.matchMode === 'insensitive') !== insensitive) continue;
      for (const raw of [entity.name, ...entity.aliases]) {
        const term = raw.trim();
        if ([...term].length < 2) continue;
        const key = insensitive ? term.toLowerCase() : term;
        if (!owners.has(key)) owners.set(key, entity);
      }
    }
    if (!owners.size) continue;
    // Longest first, so the alternation prefers "Kael Dor" to "Kael"; if the
    // longer one fails the word check the regex backtracks to the shorter.
    const alternation = [...owners.keys()]
      .sort((a, b) => b.length - a.length)
      .map(escapeRegex)
      .join('|');
    const regex = new RegExp(
      `(?<![\\p{L}\\p{N}_])(?:${alternation})(?![\\p{L}\\p{N}_])`,
      insensitive ? 'giu' : 'gu'
    );
    patterns.push({ regex, owners });
  }
  return { patterns };
}

/**
 * Every mention in `text`, in order, none overlapping.
 * @param {Matcher} matcher
 * @param {string} text
 * @returns {{ from: number, to: number, entity: Entity }[]}
 */
export function findMentions(matcher, text) {
  const hits = [];
  for (const { regex, owners } of matcher.patterns) {
    const insensitive = regex.flags.includes('i');
    for (const m of text.matchAll(regex)) {
      const entity = owners.get(insensitive ? m[0].toLowerCase() : m[0]);
      if (entity) hits.push({ from: m.index, to: m.index + m[0].length, entity });
    }
  }
  hits.sort((a, b) => a.from - b.from || b.to - b.from - (a.to - a.from));
  const kept = [];
  for (const hit of hits) {
    if (!kept.length || hit.from >= kept[kept.length - 1].to) kept.push(hit);
  }
  return kept;
}

/**
 * @param {import('@milkdown/kit/prose/model').Node} node - A textblock.
 * @param {number} pos - Its position.
 * @param {Matcher} matcher
 * @returns {Decoration[]}
 */
function blockDecorations(node, pos, matcher) {
  if (node.type.spec.code) return [];
  // One placeholder per inline leaf keeps string offsets equal to positions.
  const text = node.textBetween(0, node.content.size, undefined, '￼');
  return findMentions(matcher, text).map(({ from, to, entity }) =>
    Decoration.inline(pos + 1 + from, pos + 1 + to, {
      class: 'codex-mention',
      'data-codex-kind': entity.kind,
      title: entity.summary ? `${entity.name}: ${entity.summary}` : entity.name,
    })
  );
}

/**
 * @param {import('@milkdown/kit/prose/model').Node} doc
 * @param {Matcher | null} matcher
 */
function decorateAll(doc, matcher) {
  if (!matcher?.patterns.length) return DecorationSet.empty;
  /** @type {Decoration[]} */
  const decos = [];
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    decos.push(...blockDecorations(node, pos, matcher));
    return false;
  });
  return DecorationSet.create(doc, decos);
}

/**
 * Hands the plugin the entries to underline, or none to stop.
 * @param {import('@milkdown/kit/prose/view').EditorView} view
 * @param {Entity[]} entities
 */
export function setCodexMentions(view, entities) {
  view.dispatch(view.state.tr.setMeta(codexMentionsKey, entities));
}

export const codexMentionsPlugin = $prose(
  () =>
    new Plugin({
      key: codexMentionsKey,
      state: {
        init() {
          return { matcher: /** @type {Matcher | null} */ (null), set: DecorationSet.empty };
        },
        apply(tr, value, _old, state) {
          const entities = tr.getMeta(codexMentionsKey);
          if (entities) {
            const matcher = entities.length ? buildMatcher(entities) : null;
            return { matcher, set: decorateAll(state.doc, matcher) };
          }
          if (!tr.docChanged || !value.matcher) return value;

          // Only redo the textblocks the edit touched.
          let set = value.set.map(tr.mapping, tr.doc);
          /** @type {[number, number][]} */
          const ranges = [];
          tr.mapping.maps.forEach((map, i) => {
            map.forEach((_oldStart, _oldEnd, newStart, newEnd) => {
              const rest = tr.mapping.slice(i + 1);
              ranges.push([rest.map(newStart, -1), rest.map(newEnd, 1)]);
            });
          });
          for (const [from, to] of ranges) {
            const start = Math.max(0, Math.min(from, tr.doc.content.size));
            const end = Math.max(start, Math.min(to, tr.doc.content.size));
            tr.doc.nodesBetween(start, end, (node, pos) => {
              if (!node.isTextblock) return true;
              set = set.remove(set.find(pos, pos + node.nodeSize));
              set = set.add(tr.doc, blockDecorations(node, pos, value.matcher));
              return false;
            });
          }
          return { matcher: value.matcher, set };
        },
      },
      props: {
        decorations(state) {
          return codexMentionsKey.getState(state)?.set;
        },
      },
    })
);
