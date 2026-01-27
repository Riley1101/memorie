import { browser } from '$app/environment';

/**
 * Detects if the current platform is Mac.
 * @returns {boolean}
 */
export function isMac() {
  if (!browser) return false;
  return navigator.userAgent.toLowerCase().includes('mac');
}

/**
 * The visual label for the modifier key.
 * ⌘ on Mac, Ctrl on others.
 */
export const MOD_KEY = isMac() ? '⌘' : 'Ctrl';

/**
 * The text label for the modifier key.
 * Cmd on Mac, Ctrl on others.
 */
export const MOD_LABEL = isMac() ? 'Cmd' : 'Ctrl';

/**
 * Checks if the modifier key for the current platform is pressed.
 * @param {KeyboardEvent} e
 * @returns {boolean}
 */
export function isMod(e) {
  return isMac() ? e.metaKey : e.ctrlKey;
}
