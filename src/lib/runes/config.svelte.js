import { invoke } from '@tauri-apps/api/core';

class ConfigManager {
  /**
   * @type {{
   *   content_directory: string,
   *   undotree_dir: string,
   *   default_llm_model: string,
   *   default_llm_model_id?: string | null,
   *   system_prompt?: string | null,
   *   github_repo?: string | null,
   *   auto_push_on_exit?: boolean,
   *   ai_enabled?: boolean,
   *   provider?: 'local' | 'openrouter',
   *   openrouter_model?: string | null,
   *   dropbox_folder?: string | null,
   *   dropbox_auto_push_on_exit?: boolean,
   *   sync_provider?: 'none' | 'github' | 'dropbox'
   * } | null}
   */
  config = $state(null);

  isLoading = $state(false);
  error = $state(null);

  /** @type boolean - whether an OpenRouter API key is currently stored in the OS keyring **/
  hasOpenRouterApiKey = $state(false);

  /** @type {Array<{id: string, name: string}>} - models available on OpenRouter **/
  openRouterModels = $state([]);

  isLoadingOpenRouterModels = $state(false);

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

  /** Set the system prompt used for the AI chat. */
  async setSystemPrompt(prompt) {
    try {
      await invoke('set_system_prompt', { prompt });
      await this.getConfig();
    } catch (err) {
      console.error('Failed to set system prompt:', err);
      this.error = err;
    }
  }

  /** Toggle committing & pushing to GitHub automatically when the app closes. */
  async setAutoPushOnExit(enabled) {
    try {
      await invoke('set_auto_push_on_exit', { enabled });
      await this.getConfig();
    } catch (err) {
      console.error('Failed to set auto push on exit:', err);
      this.error = err;
    }
  }

  /** Pick which backend Cloud Sync uses: 'none', 'github' or 'dropbox'. */
  async setSyncProvider(provider) {
    try {
      await invoke('set_sync_provider', { provider });
      await this.getConfig();
    } catch (err) {
      console.error('Failed to set sync provider:', err);
      this.error = err;
    }
  }

  /** Toggle pushing writing to Dropbox automatically when the app closes. */
  async setDropboxAutoPushOnExit(enabled) {
    try {
      await invoke('set_dropbox_auto_push_on_exit', { enabled });
      await this.getConfig();
    } catch (err) {
      console.error('Failed to set Dropbox auto push on exit:', err);
      this.error = err;
    }
  }

  /** Toggle local AI (model download/load/chat). Disabled by default. */
  async setAiEnabled(enabled) {
    try {
      await invoke('set_ai_enabled', { enabled });
      await this.getConfig();
    } catch (err) {
      console.error('Failed to set AI enabled:', err);
      this.error = err;
    }
  }

  /** Switch which backend serves chat/autocomplete/grammar. RAG search stays local always. */
  async setAiProvider(provider) {
    try {
      await invoke('set_ai_provider', { provider });
      await this.getConfig();
    } catch (err) {
      console.error('Failed to set AI provider:', err);
      this.error = err;
    }
  }

  /** Set the OpenRouter model id to use (e.g. 'anthropic/claude-sonnet-4'). */
  async setOpenRouterModel(modelId) {
    try {
      await invoke('set_openrouter_model', { modelId });
      await this.getConfig();
    } catch (err) {
      console.error('Failed to set OpenRouter model:', err);
      this.error = err;
    }
  }

  /** Store the OpenRouter API key in the OS keyring. */
  async setOpenRouterApiKey(key) {
    try {
      await invoke('set_openrouter_api_key', { key });
      this.hasOpenRouterApiKey = true;
    } catch (err) {
      console.error('Failed to save OpenRouter API key:', err);
      this.error = err;
      throw err;
    }
  }

  /** Remove the stored OpenRouter API key. */
  async clearOpenRouterApiKey() {
    try {
      await invoke('clear_openrouter_api_key');
      this.hasOpenRouterApiKey = false;
      this.openRouterModels = [];
    } catch (err) {
      console.error('Failed to clear OpenRouter API key:', err);
      this.error = err;
    }
  }

  /** Refreshes whether an OpenRouter API key is currently stored. */
  async checkOpenRouterApiKey() {
    try {
      this.hasOpenRouterApiKey = await invoke('has_openrouter_api_key');
    } catch (err) {
      console.error('Failed to check OpenRouter API key:', err);
    }
  }

  /** Fetches the list of models available on OpenRouter (requires an API key to be set). */
  async fetchOpenRouterModels() {
    this.isLoadingOpenRouterModels = true;
    try {
      this.openRouterModels = await invoke('get_openrouter_models');
    } catch (err) {
      console.error('Failed to fetch OpenRouter models:', err);
      this.error = err;
    } finally {
      this.isLoadingOpenRouterModels = false;
    }
  }
}

export const configManager = new ConfigManager();
