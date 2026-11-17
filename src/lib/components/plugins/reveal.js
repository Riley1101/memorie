/**
 * Scrolls `el` into view, and keeps it there in book-length documents.
 *
 * Large documents skip rendering offscreen blocks (`content-visibility` in
 * editor.svelte), so the browser only estimates their height. Blocks that
 * render on the way to the target can then be taller or shorter than
 * estimated and push it out of place. Re-aiming for a few frames, without
 * animation, lands it where it was asked to go.
 * @param {Element | null | undefined} el
 * @param {{ block?: ScrollLogicalPosition, smooth?: boolean }} [options]
 */
export function revealElement(el, { block = 'center', smooth = false } = {}) {
  if (!el) return;
  const large = !!el.closest('.large-doc');
  el.scrollIntoView({ block, behavior: smooth && !large ? 'smooth' : 'instant' });
  if (!large) return;
  let frames = 0;
  const settle = () => {
    el.scrollIntoView({ block, behavior: 'instant' });
    if (++frames < 4) requestAnimationFrame(settle);
  };
  requestAnimationFrame(settle);
}
