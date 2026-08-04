import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';

class GitManager {
  /** @type {{ login: string, name?: string|null, avatar_url?: string|null } | null} */
  user = $state(null);

  /** @type {{ user_code: string, verification_uri: string } | null} */
  deviceCode = $state(null);

  isConnecting = $state(false);
  isCheckingSession = $state(false);

  /** @type {{ path: string, status: string }[]} */
  status = $state([]);
  isLoadingStatus = $state(false);

  isPushing = $state(false);
  pushResult = $state(null);

  isImporting = $state(false);
  importResult = $state(null);

  error = $state(null);

  _pollTimer = null;

  async checkSession() {
    this.isCheckingSession = true;
    this.error = null;
    try {
      this.user = await invoke('github_get_user');
    } catch (err) {
      console.error('Failed to check GitHub session:', err);
      this.error = err;
    } finally {
      this.isCheckingSession = false;
    }
  }

  async startLogin() {
    this.error = null;
    this.isConnecting = true;
    try {
      const code = await invoke('github_device_start');
      this.deviceCode = { user_code: code.user_code, verification_uri: code.verification_uri };
      await openUrl(code.verification_uri);
      this._poll(code.device_code, code.interval);
    } catch (err) {
      console.error('Failed to start GitHub login:', err);
      this.error = err;
      this.isConnecting = false;
    }
  }

  _poll(deviceCode, intervalSeconds) {
    clearTimeout(this._pollTimer);
    this._pollTimer = setTimeout(async () => {
      try {
        const result = await invoke('github_device_poll', { deviceCode });
        if (result.status === 'success') {
          this.user = result.user;
          this.deviceCode = null;
          this.isConnecting = false;
        } else if (result.status === 'pending') {
          this._poll(deviceCode, intervalSeconds);
        } else if (result.status === 'slow_down') {
          this._poll(deviceCode, intervalSeconds + 5);
        } else if (result.status === 'expired') {
          this.error = 'Login code expired, please try again.';
          this.deviceCode = null;
          this.isConnecting = false;
        } else {
          this.error = result.message ?? 'GitHub login failed.';
          this.deviceCode = null;
          this.isConnecting = false;
        }
      } catch (err) {
        console.error('GitHub device poll failed:', err);
        this.error = err;
        this.deviceCode = null;
        this.isConnecting = false;
      }
    }, intervalSeconds * 1000);
  }

  cancelLogin() {
    clearTimeout(this._pollTimer);
    this.deviceCode = null;
    this.isConnecting = false;
  }

  async logout() {
    try {
      await invoke('github_logout');
      this.user = null;
    } catch (err) {
      console.error('Failed to log out of GitHub:', err);
      this.error = err;
    }
  }

  async setRepo(ownerRepo) {
    this.error = null;
    try {
      await invoke('git_set_repo', { ownerRepo });
    } catch (err) {
      console.error('Failed to set GitHub repo:', err);
      this.error = err;
      throw err;
    }
  }

  /** Pull an existing GitHub repo's content into the local content directory. */
  async pull() {
    this.error = null;
    this.importResult = null;
    this.isImporting = true;
    try {
      await invoke('git_pull');
      this.importResult = 'success';
      await this.refreshStatus();
    } catch (err) {
      console.error('Failed to import from GitHub:', err);
      this.error = err;
      this.importResult = 'error';
    } finally {
      this.isImporting = false;
    }
  }

  async refreshStatus() {
    this.isLoadingStatus = true;
    try {
      this.status = await invoke('git_status');
    } catch (err) {
      console.error('Failed to load git status:', err);
      this.error = err;
    } finally {
      this.isLoadingStatus = false;
    }
  }

  async commitAndPush(message) {
    this.error = null;
    this.pushResult = null;
    this.isPushing = true;
    try {
      await invoke('git_commit_and_push', { message });
      this.pushResult = 'success';
      await this.refreshStatus();
    } catch (err) {
      console.error('Failed to commit & push:', err);
      this.error = err;
      this.pushResult = 'error';
    } finally {
      this.isPushing = false;
    }
  }
}

export const gitManager = new GitManager();
