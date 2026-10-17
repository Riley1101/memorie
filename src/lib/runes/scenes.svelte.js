import { invoke } from '@tauri-apps/api/core';
import { SvelteMap } from 'svelte/reactivity';
import { fileManager } from '$lib/runes/fs.svelte.js';
import { parseWriting } from '$lib/front-matter.js';
import { countWords, markdownToPlainText } from '$lib/runes/writing.svelte.js';

/**
 * @file Scene metadata — synopsis, status and word count — read once and
 * shared.
 *
 * The corkboard used to hold this cache privately, so the same data was
 * visible on a card and invisible on the identical writing one view away. Both
 * views read from here now, and a writing's status looks the same wherever it
 * is shown.
 */

/**
 * @typedef {Object} Scene
 * @property {number} modified - The `last_modified` this was parsed at.
 * @property {import('$lib/front-matter.js').SceneMeta} meta
 * @property {string[]} extra - Front-matter lines the app doesn't understand.
 * @property {string} body
 * @property {string} raw - Front matter exactly as it was on disk.
 * @property {string} excerpt
 * @property {number} words
 */

const EXCERPT_CHARS = 220;

class SceneCache {
  /** @type {SvelteMap<string, Scene>} */
  entries = new SvelteMap();

  /** Names being read right now, so overlapping requests don't stack up. */
  #inFlight = new Set();

  /** @param {string} name @returns {Scene | undefined} */
  get(name) {
    return this.entries.get(name);
  }

  /** Status of a writing, or '' when it has none. @param {string} name */
  statusOf(name) {
    return this.entries.get(name)?.meta.status?.trim() ?? '';
  }

  /** Word count, or null when the writing hasn't been read yet. @param {string} name */
  wordsOf(name) {
    return this.entries.get(name)?.words ?? null;
  }

  /**
   * Reads anything in `names` that is missing or out of date. Safe to call
   * from an effect: unchanged writings cost nothing.
   * @param {{ name: string, last_modified: number }[]} files
   * @returns {Promise<void>}
   */
  async load(files) {
    const stale = files.filter(
      (f) => this.entries.get(f.name)?.modified !== f.last_modified && !this.#inFlight.has(f.name)
    );
    if (stale.length === 0) return;

    for (const f of stale) this.#inFlight.add(f.name);
    const modified = new Map(stale.map((f) => [f.name, f.last_modified]));

    try {
      /** @type {[string, { Ok?: string } | string][]} */
      const results = await invoke('read_files', { names: stale.map((f) => f.name) });
      for (const [name, result] of results) {
        const content = typeof result === 'string' ? result : result?.Ok;
        if (typeof content !== 'string') continue;
        const { meta, extra, body, raw } = parseWriting(content);
        const plain = markdownToPlainText(body).replace(/\s+/g, ' ').trim();
        this.entries.set(name, {
          modified: modified.get(name) ?? 0,
          meta,
          extra,
          body,
          raw,
          excerpt: plain.length > EXCERPT_CHARS ? `${plain.slice(0, EXCERPT_CHARS)}…` : plain,
          words: countWords(plain),
        });
      }
    } finally {
      for (const f of stale) this.#inFlight.delete(f.name);
    }
  }

  /** Loads every writing under `dir`. @param {string} dir */
  loadUnder(dir) {
    return this.load(fileManager.filesUnder(dir));
  }

  /** Drops a writing, e.g. after it is renamed or deleted. @param {string} name */
  forget(name) {
    this.entries.delete(name);
  }

  /** Every status in use under `dir`, sorted. @param {string} dir */
  statusesUnder(dir) {
    // Local to this call and returned as a sorted array; nothing reacts to it.
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const seen = new Set();
    for (const file of fileManager.filesUnder(dir)) {
      const status = this.statusOf(file.name);
      if (status) seen.add(status);
    }
    return [...seen].sort();
  }
}

export const sceneCache = new SceneCache();

/**
 * Colour for a status chip or dot. Scrivener's ladder reads as progress, so
 * it gets a progression; anything the writer invents stays neutral.
 * @param {string} status
 * @returns {{ dot: string, chip: string }}
 */
export function statusTone(status) {
  switch (status.trim().toLowerCase()) {
    case 'to do':
      return { dot: 'bg-muted-foreground/40', chip: 'bg-muted text-muted-foreground' };
    case 'first draft':
      return { dot: 'bg-warning', chip: 'bg-warning/15 text-warning' };
    case 'revised draft':
      return { dot: 'bg-primary', chip: 'bg-primary/15 text-primary' };
    case 'final draft':
    case 'done':
      return { dot: 'bg-teal', chip: 'bg-teal/15 text-teal' };
    default:
      return { dot: 'bg-muted-foreground/40', chip: 'bg-muted text-muted-foreground' };
  }
}
