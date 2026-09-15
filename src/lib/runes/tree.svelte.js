/**
 * Remembers which folders are collapsed in the binder tree, per full folder
 * path (e.g. "Novel/Part 1/Chapter 3"), across visits and app restarts.
 */
const STORAGE_KEY = 'memoire.folder-collapsed';

class TreeState {
  /** Paths that are collapsed. Everything else is open. @type {Record<string, true>} */
  collapsed = $state(load());

  /** @param {string} path */
  isOpen(path) {
    return !this.collapsed[path];
  }

  /** @param {string} path @param {boolean} open */
  setOpen(path, open) {
    if (open === this.isOpen(path)) return;
    const next = { ...this.collapsed };
    if (open) delete next[path];
    else next[path] = true;
    this.collapsed = next;
    persist(next);
  }

  /** @param {string} path */
  toggle(path) {
    this.setOpen(path, !this.isOpen(path));
  }

  /** Opens `path` and every folder above it, so it can be scrolled into view. */
  reveal(path) {
    const segments = path.split('/');
    for (let i = 1; i <= segments.length; i++) {
      this.setOpen(segments.slice(0, i).join('/'), true);
    }
  }
}

function load() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function persist(value) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    // Private mode or blocked storage: collapse state just won't survive a restart.
  }
}

export const treeState = new TreeState();
