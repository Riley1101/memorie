/**
 * Application state management for the AI chat interface and other UI elements.
 */
class AppState {
  /**
   * @public
   * @type {{
   * isChatOpen: boolean,
   * isHistoryOpen: boolean
   * isSidebarOpen: boolean
   * }}
   */
  ui = $state({
    isChatOpen: false,
    isHistoryOpen: true,
    isSidebarOpen: true,
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
      isSidebarOpen: state
    };
  }
}

export let appState = new AppState();
