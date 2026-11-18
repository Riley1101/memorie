import { appState } from '$lib/runes/app.svelte.js';

/**
 * A binder's colour as a background class, for its dot or edge. Tailwind
 * needs the class names written out, so they live in a table rather than
 * being built from the colour name.
 * @type {Record<string, string>}
 */
export const BINDER_DOT = {
  teal: 'bg-binder-teal',
  primary: 'bg-binder-clay',
  info: 'bg-binder-slate',
  success: 'bg-binder-sage',
  warning: 'bg-binder-ochre',
  accent: 'bg-binder-plum',
  muted: 'bg-muted-foreground/50',
};

/**
 * The dot class for a binder; writings outside any binder get a neutral one.
 * @param {string | null | undefined} name
 */
export function binderDot(name) {
  if (!name) return BINDER_DOT.muted;
  return BINDER_DOT[appState.binderColour(name)] ?? BINDER_DOT.primary;
}
