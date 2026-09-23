import { configManager } from '$lib/runes/config.svelte.js';
import { gitManager } from '$lib/runes/git.svelte.js';
import { dropboxManager } from '$lib/runes/dropbox.svelte.js';
import { toast } from '$lib/toast.js';

/**
 * Publishing always goes to the preferred storage picked under Settings →
 * Cloud Sync — the same backend auto-push on exit uses — so the sidebar
 * button and the `:sync` command can never disagree about where writing goes.
 */

/**
 * @typedef {Object} SyncTarget
 * @property {'none' | 'github' | 'dropbox'} provider
 * @property {string | null} label - Display name, e.g. "Dropbox".
 * @property {boolean} ready - Connected and pointed at a repo/folder.
 * @property {number} pending - Files a publish would send.
 * @property {boolean} busy - A publish is in flight.
 */

/**
 * The preferred storage and its state. Reads reactive state, so it can be
 * used inside `$derived`.
 * @returns {SyncTarget}
 */
export function syncTarget() {
  const config = configManager.config;
  const provider = config?.sync_provider ?? 'none';

  if (provider === 'github') {
    return {
      provider,
      label: 'GitHub',
      ready: Boolean(gitManager.user && config?.github_repo),
      pending: gitManager.status.length,
      busy: gitManager.isPushing,
    };
  }

  if (provider === 'dropbox') {
    return {
      provider,
      label: 'Dropbox',
      ready: Boolean(dropboxManager.account && config?.dropbox_folder),
      // Push skips conflicts and Dropbox-only files, so they aren't counted.
      pending: dropboxManager.pendingPush.length,
      busy: dropboxManager.isPushing,
    };
  }

  return { provider: 'none', label: null, ready: false, pending: 0, busy: false };
}

/** Loads the preferred storage's session and pending changes. */
export async function refreshSyncStatus() {
  if (!configManager.config) await configManager.getConfig();
  const provider = configManager.config?.sync_provider ?? 'none';

  if (provider === 'github') {
    await gitManager.checkSession();
    if (syncTarget().ready) await gitManager.refreshStatus();
  } else if (provider === 'dropbox') {
    await dropboxManager.checkSession();
    if (syncTarget().ready) await dropboxManager.refreshStatus();
  }
}

/**
 * Publishes writing to the preferred storage and reports the outcome in a
 * toast. Returns whether anything was published.
 * @returns {Promise<boolean>}
 */
export async function publish() {
  let target = syncTarget();

  if (target.provider === 'none') {
    toast.info('Cloud sync is off', 'Pick GitHub or Dropbox under Settings → Cloud Sync.');
    return false;
  }

  // The session may not be loaded yet, e.g. when the app opened straight
  // into a writing and the home sidebar never mounted.
  if (!target.ready) {
    await refreshSyncStatus();
    target = syncTarget();
  }

  if (!target.ready) {
    const where = target.provider === 'github' ? 'a repository' : 'a folder';
    toast.info(
      `${target.label} isn't set up`,
      `Connect ${target.label} and choose ${where} under Settings → Cloud Sync.`
    );
    return false;
  }

  if (target.provider === 'dropbox') {
    await dropboxManager.push();
    if (dropboxManager.pushResult === 'error') {
      toast.error('Publish failed', dropboxManager.error);
      return false;
    }
    const conflicts = dropboxManager.lastSync?.conflicts ?? 0;
    if (conflicts) {
      toast.info(
        'Published to Dropbox',
        `${conflicts} file(s) changed on both sides were skipped — pull in Settings to keep both versions.`
      );
    } else {
      toast.success('Published to Dropbox');
    }
    return true;
  }

  await gitManager.commitAndPush(`Update writing — ${new Date().toLocaleString()}`);
  if (gitManager.pushResult === 'error') {
    toast.error('Publish failed', gitManager.error);
    return false;
  }
  toast.success('Published to GitHub');
  return true;
}
