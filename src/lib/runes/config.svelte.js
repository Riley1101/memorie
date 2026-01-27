import { invoke } from '@tauri-apps/api/core';

class ConfigManager {
  /**
   * @type {{
   *   content_directory: string,
   *   undotree_dir: string,
   *   default_llm_model: string
   * } | null}
   */
  config = $state(null);

  isLoading = $state(false);
  error = $state(null);

  async getConfig() {
    this.isLoading = true;
    this.error = null;
    try {
      this.config = await invoke('get_config');
    } catch (err) {
      console.error('Failed to get config:', err);
      this.error = err;
    } finally {
      this.isLoading = false;
    }
  }
}

export const configManager = new ConfigManager();
