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

export const toast = {
  /** @param {string} message @param {string} [description] */
  success(message, description) {
    sonner.success(message, description ? { description } : undefined);
  },
  /** @param {string} message @param {unknown} [err] */
  error(message, err) {
    sonner.error(message, err ? { description: errorText(err) } : undefined);
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
};
