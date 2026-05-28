/**
 * Application state management for the AI chat interface and other UI elements.
 */
class AppState {
  /** @type {('default' | 'zinc' | 'slate' | 'rose' | 'blue' | 'green' | 'violet')[]} */
  static THEME_PALETTES = ['default', 'zinc', 'slate', 'rose', 'blue', 'green', 'violet'];

  /** @type {('default' | 'minimal' | 'paper' | 'technical')[]} */
  static STYLE_FLAVOURS = ['default', 'minimal', 'paper', 'technical'];

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
   *   styleFlavour: string,
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
    theme: 'dark',
    themePalette: 'default',
    styleFlavour: 'default',
    fontSize: 18,
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

  /**
   * @param {'dark' | 'light'} theme
   */
  setTheme(theme) {
    this.ui = {
      ...this.ui,
      theme,
    };
    if (typeof window !== 'undefined') {
      localStorage.setItem('theme', theme);
    }
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
   * @param {string} flavour - One of default, minimal, paper, technical
   */
  setStyleFlavour(flavour) {
    const next = AppState.STYLE_FLAVOURS.includes(flavour) ? flavour : 'default';
    this.ui = {
      ...this.ui,
      styleFlavour: next,
    };
    if (typeof window !== 'undefined') {
      localStorage.setItem('styleFlavour', next);
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
export const STYLE_FLAVOURS = AppState.STYLE_FLAVOURS;
