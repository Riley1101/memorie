/**
 * Application state management for the AI chat interface and other UI elements.
 */
class AppState {
  /**
   * @public
   * @type {{
   *   isChatOpen: boolean,
   *   isHistoryOpen: boolean,
   *   isSidebarOpen: boolean,
   *   isCommandMenuOpen: boolean,
   *   isHelpModalOpen: boolean,
   *   theme: 'dark' | 'light',
   * }}
   */
  ui = $state({
    isChatOpen: false,
    isHistoryOpen: false,
    isSidebarOpen: false,
    isCommandMenuOpen: false,
    isHelpModalOpen: false,
    theme: 'dark',
  });

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
}

export let appState = new AppState();
