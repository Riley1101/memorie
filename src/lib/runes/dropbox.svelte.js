import { invoke } from '@tauri-apps/api/core';
import { configManager } from '$lib/runes/config.svelte.js';
import { memoryManager } from '$lib/runes/memory.svelte.js';
import { openUrl } from '@tauri-apps/plugin-opener';

const POLL_INTERVAL_MS = 1500;

class DropboxManager {
  /** @type {{ account_id: string, name: string, email?: string|null, photo_url?: string|null } | null} */
  account = $state(null);

  isConnecting = $state(false);
  isCheckingSession = $state(false);

  /** @type {{ path: string, status: string }[]} */
  status = $state([]);

  /** Files a push would upload or delete: changed only on this machine. Conflicts are skipped by push. */
  get pendingPush() {
    return this.status.filter((f) =>
      ['local_new', 'local_modified', 'local_deleted'].includes(f.status)
    );
  }
  isLoadingStatus = $state(false);

  isPushing = $state(false);
  pushResult = $state(null);

  isPulling = $state(false);
  pullResult = $state(null);

  /** @type {{ uploaded: number, downloaded: number, conflicts: number } | null} */
  lastSync = $state(null);

  error = $state(null);

  _pollTimer = null;

  async checkSession() {
    this.isCheckingSession = true;
    this.error = null;
    try {
      this.account = await invoke('dropbox_get_account');
    } catch (err) {
      console.error('Failed to check Dropbox session:', err);
      this.error = err;
    } finally {
      this.isCheckingSession = false;
    }
  }

  /**
   * Opens Dropbox's consent page in the browser. The backend listens on a
   * loopback port for the redirect, so we poll until it lands.
   */
  async startLogin() {
    this.error = null;
    this.isConnecting = true;
    try {
      const { auth_url } = await invoke('dropbox_start_login');
      await openUrl(auth_url);
      this._poll();
    } catch (err) {
      console.error('Failed to start Dropbox login:', err);
      this.error = err;
      this.isConnecting = false;
    }
  }

  _poll() {
    clearTimeout(this._pollTimer);
    this._pollTimer = setTimeout(async () => {
      try {
        const result = await invoke('dropbox_poll_login');
        if (result.status === 'success') {
          this.account = result.account;
          this.isConnecting = false;
        } else if (result.status === 'pending') {
          this._poll();
        } else if (result.status === 'expired') {
          this.error = 'Dropbox login timed out, please try again.';
          this.isConnecting = false;
        } else {
          this.error = result.message ?? 'Dropbox login failed.';
          this.isConnecting = false;
        }
      } catch (err) {
        console.error('Dropbox login poll failed:', err);
        this.error = err;
        this.isConnecting = false;
      }
    }, POLL_INTERVAL_MS);
  }

  async cancelLogin() {
    clearTimeout(this._pollTimer);
    this.isConnecting = false;
    try {
      await invoke('dropbox_cancel_login');
    } catch (err) {
      console.error('Failed to cancel Dropbox login:', err);
    }
  }

  async logout() {
    try {
      await invoke('dropbox_logout');
      this.account = null;
      this.status = [];
    } catch (err) {
      console.error('Failed to disconnect Dropbox:', err);
      this.error = err;
    }
  }

  async setFolder(folder) {
    this.error = null;
    try {
      await invoke('dropbox_set_folder', { folder });
      await configManager.getConfig();
    } catch (err) {
      console.error('Failed to set Dropbox folder:', err);
      this.error = err;
      throw err;
    }
  }

  async refreshStatus() {
    this.isLoadingStatus = true;
    try {
      this.status = await invoke('dropbox_status');
    } catch (err) {
      console.error('Failed to load Dropbox status:', err);
      this.error = err;
    } finally {
      this.isLoadingStatus = false;
    }
  }

  /** Uploads local writing Dropbox doesn't have, or has an older version of. */
  async push() {
    this.error = null;
    this.pushResult = null;
    this.isPushing = true;
    try {
      this.lastSync = await invoke('dropbox_push');
      this.pushResult = 'success';
      await this.refreshStatus();
    } catch (err) {
      console.error('Failed to push to Dropbox:', err);
      this.error = err;
      this.pushResult = 'error';
    } finally {
      this.isPushing = false;
    }
  }

  /** Downloads writing that only exists in — or differs in — Dropbox. */
  async pull() {
    this.error = null;
    this.pullResult = null;
    this.isPulling = true;
    try {
      this.lastSync = await invoke('dropbox_pull');
      this.pullResult = 'success';
      if (configManager.config?.ai_enabled) memoryManager.reindexNotes();
      await this.refreshStatus();
    } catch (err) {
      console.error('Failed to pull from Dropbox:', err);
      this.error = err;
      this.pullResult = 'error';
    } finally {
      this.isPulling = false;
    }
  }
}

export const dropboxManager = new DropboxManager();
