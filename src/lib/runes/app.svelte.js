/**
 * Application state management for the AI chat interface and other UI elements.
 */
class AppState {
  /** @type {('default' | 'zinc' | 'slate' | 'rose' | 'blue' | 'green' | 'violet')[]} */
  static THEME_PALETTES = ['default', 'zinc', 'slate', 'rose', 'blue', 'green', 'violet'];

  /** @type {('default' | 'compact')[]} */
  static DENSITY_MODES = ['default', 'compact'];

  /**
   * Reading themes for the whole app, not just the editor: typeface, page
   * background, spacing and blockquote treatment. Independent of light/dark
   * and of the accent palette below.
   * @type {{ id: string, label: string, description: string }[]}
   */
  static STYLE_FLAVOURS = [
    {
      id: 'paper',
      label: 'Paper',
      description: 'A warm page with a soft shadow and ornamental quotes. Reads like a printed manuscript.',
    },
    {
      id: 'default',
      label: 'Classic',
      description: 'Serif type on a plain background. Understated and book-like without the page effect.',
    },
    {
      id: 'minimal',
      label: 'Minimal',
      description: 'Sans-serif and airy. Closer to a plain document than a book.',
    },
    {
      id: 'technical',
      label: 'Technical',
      description: 'Compact sans-serif with code-friendly quotes. Built for notes and docs, not prose.',
    },
  ];

  /** Preset base font size (px) per density mode. */
  static DENSITY_FONT_SIZE = { default: 20, compact: 16 };

  /**
   * @public
   * @type {{
   *   isChatOpen: boolean,
   *   isHistoryOpen: boolean,
   *   isOutlineOpen: boolean,
   *   isSidebarOpen: boolean,
   *   isCommandMenuOpen: boolean,
   *   isHelpModalOpen: boolean,
   *   isNewPickerOpen: boolean,
   *   isProjectSearchOpen: boolean,
   *   isTrashOpen: boolean,
   *   exportTarget: { fileName: string | null, dir: string } | null,
   *   importTarget: { dir: string } | null,
   *   activeBinder: string | null,
   *   lastWriting: string | null,
   *   theme: 'dark' | 'light',
   *   themePreference: 'light' | 'dark' | 'system',
   *   themePalette: string,
   *   styleFlavour: string,
   *   density: 'default' | 'compact',
   *   fontSize: number,
   *   editorVersion: number,
   * }}
   */
  ui = $state({
    isChatOpen: false,
    isHistoryOpen: false,
    /** Table of contents on the right of the writing. Persisted. */
    isOutlineOpen: false,
    isSidebarOpen: false,
    isCommandMenuOpen: false,
    isHelpModalOpen: false,
    /** "New writing in…" location picker (⌘⇧N). */
    isNewPickerOpen: false,
    /** Find and replace across all writings (⌘⇧F). */
    isProjectSearchOpen: false,
    /** "Recently deleted": restore or purge trashed writings. */
    isTrashOpen: false,
    /**
     * Export dialog. `fileName` is the writing it was opened from (null from
     * the home screen), `dir` the folder or binder offered as the default scope.
     * @type {{ fileName: string | null, dir: string } | null}
     */
    exportTarget: null,
    /**
     * Import dialog; `dir` is the suggested destination. null when closed.
     * @type {{ dir: string } | null}
     */
    importTarget: null,
    /** Binder selected on the home screen; null = All Writings. Survives navigation. */
    activeBinder: /** @type {string | null} */ (null),
    /**
     * The last writing opened, across launches. A writer's first move is
     * almost always "back to what I was writing", and the app used to open on
     * the home screen with no way back but hunting for it.
     */
    lastWriting: /** @type {string | null} */ (null),
    theme: 'dark',
    /** What the user picked: 'light' | 'dark' | 'system'. `theme` is the resolved value. */
    themePreference: 'system',
    themePalette: 'default',
    // 'paper' by default: this is a fiction/long-form writing app, and it
    // should look like one from the first launch.
    styleFlavour: 'paper',
    density: 'default',
    fontSize: AppState.DENSITY_FONT_SIZE.default,
    editorVersion: 0,
  });

  /**
   * @public
   *
   * Increases the font size by 2px.
   */
  increaseFontSize() {
    this.ui.fontSize = Math.min(this.ui.fontSize + 2, 40);
    if (typeof window !== 'undefined') {
      localStorage.setItem('fontSize', this.ui.fontSize.toString());
    }
  }

  /**
   * @public
   *
   * Decreases the font size by 2px.
   */
  decreaseFontSize() {
    this.ui.fontSize = Math.max(this.ui.fontSize - 2, 12);
    if (typeof window !== 'undefined') {
      localStorage.setItem('fontSize', this.ui.fontSize.toString());
    }
  }

  /**
   * @public
   *
   * Sets the font size directly.
   * @param {number} size
   */
  setFontSize(size) {
    this.ui.fontSize = size;
    if (typeof window !== 'undefined') {
      localStorage.setItem('fontSize', size.toString());
    }
  }

  /**
   * @public
   *
   * Toggles the AI chat interface visibility.
   * @param {boolean} state - The desired state of the chat interface (open or closed).
   * @returns {void}
   */
  toggleAiChat(state) {
    this.ui.isChatOpen = state;
  }

  /**
   * @public
   *
   * Toggles the chat history panel visibility.
   */
  toggleChatHistory() {
    this.ui.isHistoryOpen = !this.ui.isHistoryOpen;
  }

  /**
   * @public
   *
   * Toggles the table of contents panel. Omit `state` to flip it.
   * @param {boolean} [state]
   */
  toggleOutline(state) {
    this.ui.isOutlineOpen = state ?? !this.ui.isOutlineOpen;
    if (typeof window !== 'undefined') {
      localStorage.setItem('outlineOpen', String(this.ui.isOutlineOpen));
    }
  }

  /**
   * @public
   * Function to toggle the sidebar visibility.
   * @param {boolean} state - The desired state of the sidebar (open or closed).
   */
  toggleSidebar(state) {
    this.ui.isSidebarOpen = state;
  }

  /**
   * @public
   * Function to toggle the command menu visibility.
   * @param {boolean} state - The desired state of the command menu (open or closed).
   */
  toggleCommandMenu(state) {
    this.ui.isCommandMenuOpen = state;
  }

  toggleHelpModal(state) {
    this.ui.isHelpModalOpen = state;
  }

  /**
   * Search and replace to start with the next time project search opens.
   * @type {{ query: string, replacement: string, binder: string | null, options?: { caseSensitive: boolean, wholeWord: boolean, regex: boolean } } | null}
   */
  projectSearchPrefill = null;

  /**
   * Opens project search on a whole-word, exact-case replace, e.g. after a
   * codex entry is renamed. With `options`, searches that way instead, and
   * shows the replace field only when there is a replacement.
   * @param {{ query: string, replacement: string, binder: string | null, options?: { caseSensitive: boolean, wholeWord: boolean, regex: boolean } }} prefill
   */
  openProjectSearch(prefill) {
    this.projectSearchPrefill = prefill;
    this.toggleProjectSearch(true);
  }

  /** @param {boolean} state */
  toggleProjectSearch(state) {
    this.ui.isProjectSearchOpen = state;
  }

  /** @param {boolean} state */
  toggleTrash(state) {
    this.ui.isTrashOpen = state;
  }

  /**
   * Remembers the writing on screen so the home screen can offer it back.
   * @param {string | null} name - Content-relative path; null forgets it.
   */
  setLastWriting(name) {
    this.ui.lastWriting = name;
    if (typeof window === 'undefined') return;
    if (name) localStorage.setItem('lastWriting', name);
    else localStorage.removeItem('lastWriting');
  }

  /** @param {{ fileName: string | null, dir: string } | null} target - null closes the dialog. */
  openExport(target) {
    this.ui.exportTarget = target;
  }

  /** @param {{ dir: string } | null} target - null closes the dialog. */
  openImport(target) {
    this.ui.importTarget = target;
  }

  /** @param {boolean} state */
  toggleNewPicker(state) {
    this.ui.isNewPickerOpen = state;
  }

  /**
   * Sets the theme preference. 'system' follows the OS via prefers-color-scheme.
   * @param {'dark' | 'light' | 'system'} preference
   */
  setTheme(preference) {
    const next = ['dark', 'light', 'system'].includes(preference) ? preference : 'system';
    this.ui.themePreference = next;
    this.ui.theme = next === 'system' ? AppState.systemTheme() : next;
    if (typeof window !== 'undefined') {
      localStorage.setItem('theme', next);
    }
  }

  /** Re-resolve the theme from the OS when the preference is 'system'. */
  syncSystemTheme() {
    if (this.ui.themePreference !== 'system') return;
    const resolved = AppState.systemTheme();
    if (resolved !== this.ui.theme) {
      this.ui.theme = resolved;
    }
  }

  /** @returns {'dark' | 'light'} */
  static systemTheme() {
    if (typeof window === 'undefined' || !window.matchMedia) return 'dark';
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  /**
   * @param {string} flavour - One of AppState.STYLE_FLAVOURS' ids.
   */
  setStyleFlavour(flavour) {
    const valid = AppState.STYLE_FLAVOURS.some((f) => f.id === flavour);
    const next = valid ? flavour : 'paper';
    this.ui.styleFlavour = next;
    if (typeof window !== 'undefined') {
      localStorage.setItem('styleFlavour', next);
    }
  }

  /**
   * Colour chosen per binder, keyed by name. Loaded in `loadBinderColours`.
   * @type {Record<string, string>}
   */
  binderColours = $state({});

  /** Palette offered for binders. Names match the tint classes in app.css. */
  static BINDER_COLOURS = ['teal', 'primary', 'info', 'success', 'warning', 'accent'];

  loadBinderColours() {
    if (typeof window === 'undefined') return;
    try {
      const saved = JSON.parse(localStorage.getItem('binderColours') ?? '{}');
      if (saved && typeof saved === 'object') this.binderColours = saved;
    } catch {
      // Nothing saved, or unreadable; the fallback colour covers it.
    }
  }

  /**
   * @param {string} binder
   * @param {string | null} colour - null goes back to the colour from the name.
   */
  setBinderColour(binder, colour) {
    const next = { ...this.binderColours };
    if (colour) next[binder] = colour;
    else delete next[binder];
    this.binderColours = next;
    if (typeof window !== 'undefined') {
      localStorage.setItem('binderColours', JSON.stringify(next));
    }
  }

  /**
   * The colour for a binder: the writer's choice, or one derived from the
   * name so binders stay distinguishable without anyone picking anything.
   * @param {string | null} binder
   */
  binderColour(binder) {
    if (!binder) return 'muted';
    const chosen = this.binderColours[binder];
    if (chosen) return chosen;
    // Stable hash, so a binder keeps its colour between launches.
    let hash = 0;
    for (let i = 0; i < binder.length; i++) hash = (hash * 31 + binder.charCodeAt(i)) >>> 0;
    return AppState.BINDER_COLOURS[hash % AppState.BINDER_COLOURS.length];
  }

  /**
   * @param {string} palette - One of default, zinc, slate, rose, blue, green, violet
   */
  setThemePalette(palette) {
    const next = /** @type {(typeof AppState.THEME_PALETTES)[number]} */ (
      AppState.THEME_PALETTES.some((p) => p === palette) ? palette : 'default'
    );
    this.ui.themePalette = next;
    if (typeof window !== 'undefined') {
      localStorage.setItem('themePalette', next);
    }
  }

  /**
   * @param {string} density - One of default, compact. Sets overall app text size and spacing.
   */
  setDensity(density) {
    const next = /** @type {(typeof AppState.DENSITY_MODES)[number]} */ (
      AppState.DENSITY_MODES.some((d) => d === density) ? density : 'default'
    );
    this.ui.density = next;
    this.ui.fontSize = AppState.DENSITY_FONT_SIZE[next];
    if (typeof window !== 'undefined') {
      localStorage.setItem('density', next);
      localStorage.setItem('fontSize', this.ui.fontSize.toString());
    }
  }

  /**
   * @public
   *
   * Increments the editor version to force a re-mount.
   */
  incrementEditorVersion() {
    this.ui.editorVersion += 1;
  }
}

export let appState = new AppState();
export const THEME_PALETTES = AppState.THEME_PALETTES;
export const STYLE_FLAVOURS = AppState.STYLE_FLAVOURS;
export const DENSITY_MODES = AppState.DENSITY_MODES;
export const DENSITY_FONT_SIZE = AppState.DENSITY_FONT_SIZE;
export const BINDER_COLOURS = AppState.BINDER_COLOURS;
