class AppState {
  /**
   * @public
   * @type {{isChatOpen: boolean}}
   */
  ui = $state({
    isChatOpen: false,
  });

  /**
   * @public
   *
   * Toggles the AI chat interface visibility.
   * @returns {void}
   */
  toggleAiChat() {
    this.ui = {
      ...this.ui,
      isChatOpen: !this.ui.isChatOpen,
    };
  }
}

export let appState = new AppState();
