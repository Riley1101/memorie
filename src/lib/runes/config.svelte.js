import { invoke } from '@tauri-apps/api/core';

class ConfigManager {
  /**
   * @type {{
   *   content_directory: string,
   *   undotree_dir: string,
   *   default_llm_model: string,
   *   default_llm_model_id?: string | null
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

  /** Set the default LLM model by id (e.g. 'qwen_2_5_1_5b_instruct'). */
  async setDefaultLlmModel(modelId) {
    try {
      await invoke('set_default_llm_model', { modelId });
      await this.getConfig();
    } catch (err) {
      console.error('Failed to set default model:', err);
      this.error = err;
    }
  }
}

export const configManager = new ConfigManager();
