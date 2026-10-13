/**
 * @file Codex names and aliases, underlined where they appear in the text.
 *
 * The matching rules are the backend's (`codex::Matcher`): whole words only,
 * each entry's own case mode, and the longest name wins where two overlap
 * ("Iron Citadel" over "Citadel"). They live here rather than in the Milkdown
 * plugin — which is where they started — so that both editing surfaces mention
 * the same things, and so this path does not load ProseMirror to underline a
 * name. The plugin imports them back.
 *
 * A mention is not in the document: it is a decoration, so it is not in the
 * Markdown, not in the undo history, and recomputed from the text rather than
 * mapped through edits. See `decorations.js`.
 */

import { flattenBlocks, textOfRuns } from './decorations.js';

/** @typedef {import('$lib/runes/codex.svelte.js').Entity} Entity */
/** @typedef {{ regex: RegExp, owners: Map<string, Entity> }} Pattern */
/** @typedef {{ patterns: Pattern[] }} Matcher */

/** @param {string} s */
function escapeRegex(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Builds one pattern per case mode, each alternating over every name and alias
 * that mode covers.
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
  /** @type {{ from: number, to: number, entity: Entity }[]} */
  const hits = [];
  for (const { regex, owners } of matcher.patterns) {
    const insensitive = regex.flags.includes('i');
    for (const m of text.matchAll(regex)) {
      const entity = owners.get(insensitive ? m[0].toLowerCase() : m[0]);
      if (entity) hits.push({ from: m.index, to: m.index + m[0].length, entity });
    }
  }
  hits.sort((a, b) => a.from - b.from || b.to - b.from - (a.to - a.from));
  /** @type {{ from: number, to: number, entity: Entity }[]} */
  const kept = [];
  for (const hit of hits) {
    if (!kept.length || hit.from >= kept[kept.length - 1].to) kept.push(hit);
  }
  return kept;
}

/**
 * Decorations for every mention in a document, with the entry's summary as the
 * hover text.
 *
 * Code blocks are left alone: a name inside a fence is code, not a mention.
 * @param {import('$lib/rust-editor.js').EditorBlock[]} blocks
 * @param {Entity[]} entities
 * @returns {import('./decorations.js').Decoration[]}
 */
export function mentionDecorations(blocks, entities) {
  if (!entities?.length) return [];
  const matcher = buildMatcher(entities);
  if (!matcher.patterns.length) return [];

  /** @type {import('./decorations.js').Decoration[]} */
  const out = [];
  for (const block of flattenBlocks(blocks)) {
    if (block.kind === 'codeBlock' || !block.runs) continue;
    const text = textOfRuns(block.runs);
    if (!text) continue;
    for (const { from, to, entity } of findMentions(matcher, text)) {
      out.push({
        block: block.id,
        start: from,
        end: to,
        class: 'deco-mention',
        attrs: {
          'data-codex-kind': entity.kind,
          'data-codex-name': entity.name,
          title: entity.summary ? `${entity.name}: ${entity.summary}` : entity.name,
        },
      });
    }
  }
  return out;
}
