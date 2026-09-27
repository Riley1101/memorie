import { invoke } from '@tauri-apps/api/core';
import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { toast } from '$lib/toast.js';

/**
 * @file The codex: sheets for a story's characters, locations, factions and items.
 * Entries are markdown files under `Codex/` folders; see `src-tauri/src/codex.rs`.
 */

/**
 * @typedef {'character' | 'location' | 'faction' | 'item'} EntityKind
 * @typedef {'case-sensitive' | 'insensitive' | 'off'} MatchMode
 * @typedef {{ key: string, value: string }} Field
 * @typedef {{ to: string, kind: string }} Relationship
 *
 * @typedef {object} Entity
 * @property {string} path - Content-relative file, e.g. `Novel/Codex/Characters/Aria.md`.
 * @property {string | null} scope - The binder it belongs to; null for the shared codex.
 * @property {string} name
 * @property {EntityKind} kind
 * @property {string[]} aliases
 * @property {string} summary
 * @property {Field[]} fields
 * @property {Relationship[]} relationships
 * @property {MatchMode} matchMode
 * @property {boolean} pinned - Always given to the assistant in this binder.
 * @property {string} notes
 *
 * @typedef {{ path: string, count: number, first: number }} Mention
 * @typedef {{ name: string, counts: Record<string, number> }} SceneMentions
 * @typedef {{ entities: Entity[], scenes: SceneMentions[] }} Matrix
 * @typedef {{ summary: string, fields: Record<string, string> }} SheetDraft
 */

/**
 * Kinds in display order, each with the fields a new sheet starts with.
 * @type {{ id: EntityKind, label: string, plural: string, fields: string[] }[]}
 */
export const KINDS = [
  {
    id: 'character',
    label: 'Character',
    plural: 'Characters',
    fields: ['role', 'appearance', 'personality', 'goals', 'arc'],
  },
  {
    id: 'location',
    label: 'Location',
    plural: 'Locations',
    fields: ['region', 'description', 'atmosphere'],
  },
  { id: 'faction', label: 'Faction', plural: 'Factions', fields: ['leader', 'goals', 'members'] },
  { id: 'item', label: 'Item', plural: 'Items', fields: ['owner', 'description', 'properties'] },
];

/** @param {EntityKind} kind */
export function kindInfo(kind) {
  return KINDS.find((k) => k.id === kind) ?? KINDS[0];
}

/**
 * A blank sheet of `kind`, with that kind's usual fields ready to fill in.
 * @param {EntityKind} kind
 * @param {string | null} scope
 * @param {string} [name]
 * @returns {Entity}
 */
export function blankEntity(kind, scope, name = '') {
  return {
    path: '',
    scope,
    name,
    kind,
    aliases: [],
    summary: '',
    fields: kindInfo(kind).fields.map((key) => ({ key, value: '' })),
    relationships: [],
    matchMode: 'case-sensitive',
    pinned: false,
    notes: '',
  };
}

/**
 * Link to the codex page. Empty values are left out.
 * @param {{ binder?: string | null, entry?: string, tab?: string, new?: string, name?: string }} [query]
 */
export function codexHref(query = {}) {
  const qs = Object.entries(query)
    .filter(([, v]) => v)
    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(/** @type {string} */ (v))}`)
    .join('&');
  return `${resolve('/codex')}${qs ? `?${qs}` : ''}`;
}

/**
 * Opens the codex page.
 * @param {Parameters<typeof codexHref>[0]} [query]
 */
export function openCodex(query) {
  // The path goes through resolve() in codexHref; a query string can't.
  // eslint-disable-next-line svelte/no-navigation-without-resolve
  goto(codexHref(query));
}

/**
 * True for a content-relative file in a codex folder (`Codex/...` or `Binder/Codex/...`).
 * Mirrors `codex::is_codex_name` in the backend.
 * @param {string} name
 */
export function isCodexPath(name) {
  const dirs = name.split('/').slice(0, -1);
  return dirs[0] === 'Codex' || dirs[1] === 'Codex';
}

/**
 * Opens a codex entry by its path: `Novel/Codex/Characters/Aria.md` is in binder "Novel".
 * @param {string} path
 */
export function openCodexEntry(path) {
  openCodex({ binder: path.startsWith('Codex/') ? null : path.split('/')[0], entry: path });
}

class CodexManager {
  /**
   * Entries per binder ('' for the shared codex alone), as last loaded.
   * @type {Record<string, Entity[]>}
   */
  byBinder = $state({});

  /** @type {Record<string, boolean>} */
  loading = $state({});

  /**
   * Entries a binder can see: its own and the shared ones.
   * @param {string | null} binder
   * @returns {Entity[]}
   */
  entities(binder) {
    return this.byBinder[binder ?? ''] ?? [];
  }

  /**
   * Loads (or reloads) a binder's entries.
   * @param {string | null} binder
   */
  async load(binder) {
    const key = binder ?? '';
    this.loading[key] = true;
    try {
      this.byBinder[key] = await invoke('codex_list', { binder: binder || null });
    } catch (err) {
      console.error('Could not load the codex:', err);
      toast.error('Could not load the codex', err);
    } finally {
      this.loading[key] = false;
    }
  }

  /** Reloads every binder loaded so far, e.g. after a shared entry changes. */
  async reloadAll() {
    await Promise.all(Object.keys(this.byBinder).map((key) => this.load(key || null)));
  }

  /**
   * Creates or updates an entry. `originalPath` is where it was loaded from.
   * @param {Entity} entity
   * @param {string | null} originalPath
   * @returns {Promise<Entity | null>}
   */
  async save(entity, originalPath) {
    try {
      const saved = /** @type {Entity} */ (
        await invoke('codex_save', {
          entity: {
            ...entity,
            aliases: entity.aliases.map((a) => a.trim()).filter(Boolean),
          },
          originalPath: originalPath || null,
        })
      );
      await this.reloadAll();
      return saved;
    } catch (err) {
      toast.error('Could not save the codex entry', err);
      return null;
    }
  }

  /** @param {Entity} entity */
  async remove(entity) {
    try {
      await invoke('codex_delete', { path: entity.path });
      await this.reloadAll();
      return true;
    } catch (err) {
      toast.error(`Could not delete “${entity.name}”`, err);
      return false;
    }
  }

  /**
   * Which of a binder's entries `text` mentions, most mentioned first.
   * @param {string | null} binder
   * @param {string} text
   * @returns {Promise<Mention[]>}
   */
  async detect(binder, text) {
    return await invoke('codex_detect', { binder: binder || null, text });
  }

  /**
   * Suggestions for an entry's empty fields (and summary, if empty) from the
   * writings that mention it. Nothing is saved.
   * @param {string} path
   * @param {string[]} keys
   * @returns {Promise<SheetDraft>}
   */
  async draft(path, keys) {
    return await invoke('codex_draft', { path, keys });
  }

  /**
   * How often each entry is mentioned in each of a binder's writings.
   * @param {string} binder
   * @returns {Promise<Matrix>}
   */
  async matrix(binder) {
    return await invoke('codex_matrix', { binder });
  }
}

export const codexManager = new CodexManager();

/**
 * Entries the chat leaves out of, or always puts into, the next replies, by name.
 * Cleared with a new chat.
 */
class CodexChatState {
  /** @type {string[]} */
  exclude = $state([]);
  /** @type {string[]} */
  pin = $state([]);

  /** @param {string} name */
  toggleExclude(name) {
    this.pin = this.pin.filter((n) => n !== name);
    this.exclude = this.exclude.includes(name)
      ? this.exclude.filter((n) => n !== name)
      : [...this.exclude, name];
  }

  /** @param {string} name */
  togglePin(name) {
    this.exclude = this.exclude.filter((n) => n !== name);
    this.pin = this.pin.includes(name) ? this.pin.filter((n) => n !== name) : [...this.pin, name];
  }

  reset() {
    this.exclude = [];
    this.pin = [];
  }
}

export const codexChat = new CodexChatState();
