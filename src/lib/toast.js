import { toast as sonner } from 'svelte-sonner';

/**
 * Thin wrapper around svelte-sonner so callers don't depend on the library
 * directly. Errors coming from Tauri are often plain strings or objects with a
 * `message`; normalise them to a readable line.
 * @param {unknown} err
 * @returns {string}
 */
export function errorText(err) {
  if (!err) return 'Something went wrong.';
  if (typeof err === 'string') return err;
  if (typeof err === 'object' && 'message' in err && typeof err.message === 'string') {
    return err.message;
  }
  return String(err);
}

/**
 * A short sentence written for people, as opposed to a raw backend error.
 * @param {unknown} err
 */
function isHint(err) {
  return typeof err === 'string' && err.length <= 120 && /[.!?]$/.test(err.trim()) && !/error|panicked|os error|\(code/i.test(err);
}

/** @param {unknown} err */
function copyDetailsAction(err) {
  return {
    label: 'Copy error details',
    onClick: () => {
      navigator.clipboard?.writeText(errorText(err)).catch(() => {});
    },
  };
}

export const toast = {
  /** @param {string} message @param {string} [description] */
  success(message, description) {
    sonner.success(message, description ? { description } : undefined);
  },
  /**
   * An error toast. Hints written for people (a short sentence ending in a
   * full stop) show as the description; anything else is a raw backend error,
   * which stays out of sight behind a "Copy error details" button.
   * @param {string} message @param {unknown} [err]
   */
  error(message, err) {
    if (!err) {
      sonner.error(message);
      return;
    }
    if (isHint(err)) {
      sonner.error(message, { description: /** @type {string} */ (err) });
      return;
    }
    sonner.error(message, { action: copyDetailsAction(err) });
  },
  /**
   * An error that stays until it is fixed or dismissed, with a way to try
   * again and a way to copy the raw details for a bug report.
   * @param {string} message
   * @param {{ id: string, description?: string, onRetry: () => void, err?: unknown }} opts
   */
  errorWithRetry(message, { id, description, onRetry, err }) {
    sonner.error(message, {
      id,
      description,
      duration: Number.POSITIVE_INFINITY,
      action: { label: 'Try again', onClick: onRetry },
      cancel: err ? copyDetailsAction(err) : undefined,
    });
  },
  /** @param {string} message @param {string} [description] */
  info(message, description) {
    sonner(message, description ? { description } : undefined);
  },
  /**
   * A success toast with a button, e.g. "Show in Finder".
   * @param {string} message @param {string} label @param {() => void} onClick @param {string} [description]
   */
  successWithAction(message, label, onClick, description) {
    sonner.success(message, { description, action: { label, onClick } });
  },
  /**
   * A toast that stays until it is acted on or dismissed, e.g. "an update is
   * ready — restart". `id` lets a later call replace or dismiss the same one.
   * @param {string} message
   * @param {{ id?: string, description?: string, label: string, onClick: () => void }} opts
   */
  persistentWithAction(message, { id, description, label, onClick }) {
    sonner(message, {
      id,
      description,
      duration: Number.POSITIVE_INFINITY,
      action: { label, onClick },
    });
  },
  /** @param {string} id - Dismisses the toast previously shown with this id. */
  dismiss(id) {
    sonner.dismiss(id);
  },
};
