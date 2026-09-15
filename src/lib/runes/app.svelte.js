/**
 * Application state management for the AI chat interface and other UI elements.
 */
class AppState {
  /** @type {('default' | 'zinc' | 'slate' | 'rose' | 'blue' | 'green' | 'violet')[]} */
  static THEME_PALETTES = ['default', 'zinc', 'slate', 'rose', 'blue', 'green', 'violet'];

  /** @type {('default' | 'compact')[]} */
  static DENSITY_MODES = ['default', 'compact'];

  /** Preset base font size (px) per density mode. */
  static DENSITY_FONT_SIZE = { default: 20, compact: 16 };

  /**
   * @public
   * @type {{
   *   isChatOpen: boolean,
   *   isHistoryOpen: boolean,
   *   isSidebarOpen: boolean,
   *   isCommandMenuOpen: boolean,
   *   isHelpModalOpen: boolean,
   *   theme: 'dark' | 'light',
   *   themePalette: string,
   *   density: 'default' | 'compact',
   *   fontSize: number,
   *   editorVersion: number,
   * }}
   */
  ui = $state({
    isChatOpen: false,
    isHistoryOpen: false,
    isSidebarOpen: false,
    isCommandMenuOpen: false,
    isHelpModalOpen: false,
    /** "New writing in…" location picker (⌘⇧N). */
    isNewPickerOpen: false,
    /** Binder selected on the home screen; null = All Writings. Survives navigation. */
    activeBinder: /** @type {string | null} */ (null),
    theme: 'dark',
    /** What the user picked: 'light' | 'dark' | 'system'. `theme` is the resolved value. */
    themePreference: 'system',
    themePalette: 'default',
    styleFlavour: 'minimal',
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
    this.ui = {
      ...this.ui,
      fontSize: Math.min(this.ui.fontSize + 2, 40),
    };
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
    this.ui = {
      ...this.ui,
      fontSize: Math.max(this.ui.fontSize - 2, 12),
    };
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
    this.ui = {
      ...this.ui,
      fontSize: size,
    };
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
    console.debug('[DEBUG toggleAiChat] setting isChatOpen=%s', state);
    this.ui = {
      ...this.ui,
      isChatOpen: state,
    };
  }

  /**
   * @public
   *
   * Toggles the chat history panel visibility.
   */
  toggleChatHistory() {
    this.ui = {
      ...this.ui,
      isHistoryOpen: !this.ui.isHistoryOpen,
    };
  }

  /**
   * @public
   * Function to toggle the sidebar visibility.
   * @param {boolean} state - The desired state of the sidebar (open or closed).
   */
  toggleSidebar(state) {
    this.ui = {
      ...this.ui,
      isSidebarOpen: state,
    };
  }

  /**
   * @public
   * Function to toggle the command menu visibility.
   * @param {boolean} state - The desired state of the command menu (open or closed).
   */
  toggleCommandMenu(state) {
    this.ui = {
      ...this.ui,
      isCommandMenuOpen: state,
    };
  }

  toggleHelpModal(state) {
    this.ui = {
      ...this.ui,
      isHelpModalOpen: state,
    };
  }

  /** @param {boolean} state */
  toggleNewPicker(state) {
    this.ui = {
      ...this.ui,
      isNewPickerOpen: state,
    };
  }

  /**
   * Sets the theme preference. 'system' follows the OS via prefers-color-scheme.
   * @param {'dark' | 'light' | 'system'} preference
   */
  setTheme(preference) {
    const next = ['dark', 'light', 'system'].includes(preference) ? preference : 'system';
    this.ui = {
      ...this.ui,
      themePreference: next,
      theme: next === 'system' ? AppState.systemTheme() : next,
    };
    if (typeof window !== 'undefined') {
      localStorage.setItem('theme', next);
    }
  }

  /** Re-resolve the theme from the OS when the preference is 'system'. */
  syncSystemTheme() {
    if (this.ui.themePreference !== 'system') return;
    const resolved = AppState.systemTheme();
    if (resolved !== this.ui.theme) {
      this.ui = { ...this.ui, theme: resolved };
    }
  }

  /** @returns {'dark' | 'light'} */
  static systemTheme() {
    if (typeof window === 'undefined' || !window.matchMedia) return 'dark';
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  /**
   * @param {string} palette - One of default, zinc, slate, rose, blue, green, violet
   */
  setThemePalette(palette) {
    const next = AppState.THEME_PALETTES.includes(palette) ? palette : 'default';
    this.ui = {
      ...this.ui,
      themePalette: next,
    };
    if (typeof window !== 'undefined') {
      localStorage.setItem('themePalette', next);
    }
  }

  /**
   * @param {string} density - One of default, compact. Sets overall app text size and spacing.
   */
  setDensity(density) {
    const next = AppState.DENSITY_MODES.includes(density) ? density : 'default';
    this.ui = {
      ...this.ui,
      density: next,
      fontSize: AppState.DENSITY_FONT_SIZE[next],
    };
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
    this.ui = {
      ...this.ui,
      editorVersion: this.ui.editorVersion + 1,
    };
  }
}

export let appState = new AppState();
export const THEME_PALETTES = AppState.THEME_PALETTES;
export const DENSITY_MODES = AppState.DENSITY_MODES;
export const DENSITY_FONT_SIZE = AppState.DENSITY_FONT_SIZE;
