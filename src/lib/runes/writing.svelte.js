/**
 * @file Writer-facing preferences and progress: spellcheck, word counts,
 * daily and per-binder goals, and focus mode. Everything here is a local UI
 * preference, persisted to localStorage like the rest of the app's UI state.
 */

const STORAGE_KEY = 'writing';

/** Days of history kept in the progress log. */
const LOG_DAYS = 90;

/**
 * How paragraphs read in the editor and in most export formats.
 * - 'spaced': a blank gap between paragraphs, no indent (blog / doc style).
 * - 'indented': first-line indent, no gap, the way a printed manuscript reads.
 */
export const PARAGRAPH_STYLES = /** @type {const} */ (['spaced', 'indented']);

const segmenter =
  typeof Intl !== 'undefined' && 'Segmenter' in Intl
    ? new Intl.Segmenter(undefined, { granularity: 'word' })
    : null;

/**
 * Counts words the way a writer would. Uses Intl.Segmenter so languages
 * without spaces (Japanese, Chinese, Thai) still count sensibly.
 * @param {string} text
 * @returns {number}
 */
export function countWords(text) {
  if (!text) return 0;
  if (!segmenter) return text.split(/\s+/).filter(Boolean).length;
  let count = 0;
  for (const s of segmenter.segment(text)) {
    if (s.isWordLike) count++;
  }
  return count;
}

/**
 * Strips Markdown syntax that shouldn't count as words (links keep their
 * text, images and code fences go).
 * @param {string} markdown
 * @returns {string}
 */
export function markdownToPlainText(markdown) {
  return markdown
    .replace(/^---\r?\n[\s\S]*?\r?\n---[ \t]*(?:\r?\n|$)/, '')
    .replace(/```[\s\S]*?```/g, '')
    .replace(/!\[[^\]]*\]\([^)]*\)/g, '')
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[#>*_`~|-]/g, ' ');
}

const DAY_MS = 24 * 60 * 60 * 1000;

/** en-CA formats dates as YYYY-MM-DD, in the local time zone. */
const dayFormat = new Intl.DateTimeFormat('en-CA', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
});

/**
 * @param {number} [timestamp]
 * @returns {string} Local date as YYYY-MM-DD, which also sorts chronologically.
 */
export function todayKey(timestamp = Date.now()) {
  return dayFormat.format(timestamp);
}

class WritingState {
  /** Browser spellcheck in the editor. */
  spellcheck = $state(true);

  /** Words per day; 0 means no goal. */
  dailyGoal = $state(0);

  /**
   * Target word count per binder (top-level folder).
   * @type {Record<string, number>}
   */
  projectGoals = $state({});

  /**
   * Net words written per local day.
   * @type {Record<string, number>}
   */
  log = $state({});

  /** Distraction-free writing: chrome hidden, current paragraph highlighted, caret centred. */
  focusMode = $state(false);

  /** @type {(typeof PARAGRAPH_STYLES)[number]} */
  paragraphStyle = $state('spaced');

  /** Live counts for the open document. */
  docWords = $state(0);
  docChars = $state(0);
  selectionWords = $state(0);

  /** Last count seen for the open document; null until the editor reports one. */
  #baseline = /** @type {number | null} */ (null);

  /** @type {ReturnType<typeof setTimeout> | null} */
  #persistTimer = null;

  /** Today's date key; refreshed every minute so the count rolls over at midnight. */
  #today = $state(todayKey());

  /** @type {ReturnType<typeof setInterval> | null} */
  #dayTimer = null;

  todayWords = $derived(Math.max(0, this.log[this.#today] ?? 0));

  load() {
    if (!this.#dayTimer) {
      this.#dayTimer = setInterval(() => {
        const key = todayKey();
        if (key !== this.#today) this.#today = key;
      }, 60_000);
    }
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw);
      if (typeof saved.spellcheck === 'boolean') this.spellcheck = saved.spellcheck;
      if (Number.isFinite(saved.dailyGoal)) this.dailyGoal = saved.dailyGoal;
      if (saved.projectGoals && typeof saved.projectGoals === 'object') {
        this.projectGoals = saved.projectGoals;
      }
      if (saved.log && typeof saved.log === 'object') this.log = saved.log;
      if (PARAGRAPH_STYLES.includes(saved.paragraphStyle)) this.paragraphStyle = saved.paragraphStyle;
    } catch (e) {
      console.warn('Could not load writing preferences:', e);
    }
  }

  #persist() {
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          spellcheck: this.spellcheck,
          dailyGoal: this.dailyGoal,
          projectGoals: this.projectGoals,
          log: this.log,
          paragraphStyle: this.paragraphStyle,
        })
      );
    } catch (e) {
      console.warn('Could not save writing preferences:', e);
    }
  }

  /** Progress changes on every keystroke; write it out at most once a second. */
  #persistSoon() {
    if (this.#persistTimer) return;
    this.#persistTimer = setTimeout(() => {
      this.#persistTimer = null;
      this.#persist();
    }, 1000);
  }

  /** @param {(typeof PARAGRAPH_STYLES)[number]} style */
  setParagraphStyle(style) {
    this.paragraphStyle = PARAGRAPH_STYLES.includes(style) ? style : 'spaced';
    this.#persist();
  }

  /** @param {boolean} enabled */
  setSpellcheck(enabled) {
    this.spellcheck = enabled;
    this.#persist();
  }

  /** @param {number} words */
  setDailyGoal(words) {
    this.dailyGoal = Math.max(0, Math.round(words) || 0);
    this.#persist();
  }

  /**
   * @param {string} binder
   * @param {number} words - 0 clears the goal.
   */
  setProjectGoal(binder, words) {
    const next = { ...this.projectGoals };
    const value = Math.max(0, Math.round(words) || 0);
    if (value) next[binder] = value;
    else delete next[binder];
    this.projectGoals = next;
    this.#persist();
  }

  /** @param {boolean} [state] - Omit to flip. */
  toggleFocusMode(state) {
    this.focusMode = state ?? !this.focusMode;
  }

  /**
   * Called when an editor mounts: its first count is the baseline, so opening
   * or restoring a document doesn't count as writing.
   */
  resetBaseline() {
    this.#baseline = null;
    this.docWords = 0;
    this.docChars = 0;
    this.selectionWords = 0;
  }

  /**
   * Reports the open document's text. The difference from the previous count
   * is added to today's progress.
   * @param {string} text
   */
  updateDocument(text) {
    const words = countWords(text);
    this.docWords = words;
    this.docChars = text.replace(/\s/g, '').length;

    if (this.#baseline === null) {
      this.#baseline = words;
      return;
    }
    const delta = words - this.#baseline;
    this.#baseline = words;
    if (delta === 0) return;

    const key = todayKey();
    this.#today = key;
    // Never below zero: cutting old text shouldn't hide the words written today.
    const next = { ...this.log, [key]: Math.max(0, (this.log[key] ?? 0) + delta) };
    const cutoff = todayKey(Date.now() - LOG_DAYS * DAY_MS);
    for (const day of Object.keys(next)) {
      if (day < cutoff) delete next[day];
    }
    this.log = next;
    this.#persistSoon();
  }

  /** @param {string} text - Currently selected text, or '' for none. */
  updateSelection(text) {
    this.selectionWords = text ? countWords(text) : 0;
  }
}

export const writingState = new WritingState();
