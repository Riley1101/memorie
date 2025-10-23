class AppState {

  isAiChatOpen = $state(false);

  /**
   * Toggles the AI chat interface visibility.
   * @returns {void}
   */
  toggleAiChat() {
    console.log("Toggling AI Chat Interface");
    this.isAiChatOpen = !this.isAiChatOpen;
    console.log(this.isAiChatOpen)
  }
}

export let appState = new AppState();
