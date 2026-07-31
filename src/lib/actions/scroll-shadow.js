/**
 * Svelte action: calls `onChange({ canScrollUp, canScrollDown })` whenever the
 * nearest scrollable viewport inside `node` moves. Used to drive top/bottom
 * fade indicators on scrollable panels ("more content above/below" cues).
 * @param {HTMLElement} node
 * @param {(state: { canScrollUp: boolean, canScrollDown: boolean }) => void} onChange
 */
export function scrollShadow(node, onChange) {
  const viewport = node.querySelector('[data-slot="scroll-area-viewport"]') ?? node;

  function update() {
    onChange({
      canScrollUp: viewport.scrollTop > 4,
      canScrollDown: viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight > 4
    });
  }

  viewport.addEventListener('scroll', update, { passive: true });
  const resizeObserver = new ResizeObserver(update);
  resizeObserver.observe(viewport);
  update();

  return {
    destroy() {
      viewport.removeEventListener('scroll', update);
      resizeObserver.disconnect();
    }
  };
}
