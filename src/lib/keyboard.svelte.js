import { browser } from '$app/env';

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
 * @param {KeyboardEvent | MouseEvent} e
 * @returns {boolean}
 */
export function isMod(e) {
  return isMac() ? e.metaKey : e.ctrlKey;
}

/** Shift as shortcuts on this platform write it: ⇧ on Mac, "Shift" elsewhere. */
export const SHIFT_KEY = isMac() ? '⇧' : 'Shift';

/** Option/Alt as shortcuts on this platform write it: ⌥ on Mac, "Alt" elsewhere. */
export const ALT_KEY = isMac() ? '⌥' : 'Alt';

/**
 * A shortcut label in the platform's own style: "⌘⇧O" on Mac, where the
 * symbols read run together, and "Ctrl+Shift+O" elsewhere, where "CtrlShiftO"
 * would not. The modifier key is always first.
 * @param {...('shift' | 'alt' | string)} keys - Extra modifiers by name, then the key.
 * @returns {string}
 */
export function shortcut(...keys) {
  const parts = [
    MOD_KEY,
    ...keys.map((k) => (k === 'shift' ? SHIFT_KEY : k === 'alt' ? ALT_KEY : k)),
  ];
  return parts.join(isMac() ? '' : '+');
}
