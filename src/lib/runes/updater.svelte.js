import { check } from '@tauri-apps/plugin-updater';
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { toast, errorText } from '$lib/toast.js';

/** How often a running app looks for a new release. */
const RECHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
/** Lets the window and the note index settle before the first check. */
const FIRST_CHECK_DELAY_MS = 10 * 1000;
const READY_TOAST_ID = 'memoire-update-ready';

/**
 * Checks GitHub Releases for a newer Memoire, downloads it in the background
 * and offers a restart once it's on disk. Nothing installs until the writer
 * says so, so an update never interrupts a writing session.
 */
class UpdaterManager {
  /** @type {string | null} */
  currentVersion = $state(null);

  /** @type {'idle' | 'checking' | 'downloading' | 'ready' | 'installing' | 'up-to-date' | 'error'} */
  status = $state('idle');

  /** @type {{ version: string, notes?: string, date?: string } | null} */
  available = $state(null);

  /** Download progress from 0 to 1, or null when the size is unknown. */
  progress = $state(null);

  /** @type {Date | null} */
  lastChecked = $state(null);

  error = $state(null);

  /** @type {import('@tauri-apps/plugin-updater').Update | null} */
  _update = null;
  _timer = null;
  _started = false;

  get isBusy() {
    return ['checking', 'downloading', 'installing'].includes(this.status);
  }

  /** Starts background checks. Skipped in dev builds, which aren't signed releases. */
  start() {
    if (this._started) return;
    this._started = true;
    getVersion()
      .then((v) => (this.currentVersion = v))
      .catch(() => {});
    if (import.meta.env.DEV) return;

    setTimeout(() => this.check({ silent: true }), FIRST_CHECK_DELAY_MS);
    this._timer = setInterval(() => this.check({ silent: true }), RECHECK_INTERVAL_MS);
  }

  /**
   * Looks for an update and, if there is one, downloads it.
   * @param {{ silent?: boolean }} [opts] silent: no toasts for "up to date" or errors
   */
  async check({ silent = false } = {}) {
    if (this.isBusy) return;
    if (this.status === 'ready') {
      if (!silent) this._announceReady();
      return;
    }

    this.status = 'checking';
    this.error = null;
    try {
      const update = await check();
      this.lastChecked = new Date();
      if (!update) {
        this.status = 'up-to-date';
        if (!silent) toast.info("You're on the latest version.");
        return;
      }
      this.available = { version: update.version, notes: update.body, date: update.date };
      await this._download(update);
    } catch (err) {
      console.error('Update check failed:', err);
      this.status = 'error';
      this.error = errorText(err);
      if (!silent) toast.error("Couldn't check for updates.", this.error);
    }
  }

  /** @param {import('@tauri-apps/plugin-updater').Update} update */
  async _download(update) {
    this.status = 'downloading';
    this.progress = null;
    let total = 0;
    let received = 0;
    await update.download((event) => {
      if (event.event === 'Started') {
        total = event.data.contentLength ?? 0;
        this.progress = total ? 0 : null;
      } else if (event.event === 'Progress') {
        received += event.data.chunkLength;
        if (total) this.progress = Math.min(received / total, 1);
      } else if (event.event === 'Finished') {
        this.progress = 1;
      }
    });
    this._update = update;
    this.status = 'ready';
    this._announceReady();
  }

  _announceReady() {
    if (!this.available) return;
    toast.persistentWithAction(`Memoire ${this.available.version} is ready`, {
      id: READY_TOAST_ID,
      description: 'Restart to finish updating. Your writing is saved.',
      label: 'Restart',
      onClick: () => this.installAndRestart(),
    });
  }

  /** Runs the on-exit sync, installs the downloaded update and relaunches. */
  async installAndRestart() {
    if (!this._update || this.status === 'installing') return;
    this.status = 'installing';
    toast.dismiss(READY_TOAST_ID);
    try {
      // Same auto-push the window-close handler does; on Windows `install()`
      // quits the app straight away, so it has to happen first.
      await invoke('prepare_for_update');
      await this._update.install();
      await invoke('restart_app');
    } catch (err) {
      console.error('Update install failed:', err);
      this.status = 'error';
      this.error = errorText(err);
      toast.error("Couldn't install the update.", this.error);
    }
  }
}

export const updaterManager = new UpdaterManager();
