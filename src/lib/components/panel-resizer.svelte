<script>
  /**
   * The grab strip along a panel's inner edge. The panel owns its width; this
   * only reports the next one, clamped, so a panel can remember it however it
   * likes. Dragging works with a pointer, and the arrow keys move it too, so
   * the width is reachable without a mouse.
   *
   * @type {{
   *   width: number,
   *   min?: number,
   *   max?: number,
   *   label: string,
   *   onresize: (width: number) => void,
   * }}
   */
  let { width, min = 240, max = 720, label, onresize } = $props();

  let dragging = $state(false);

  /** @param {number} next */
  function report(next) {
    onresize(Math.round(Math.min(max, Math.max(min, next))));
  }

  /** @param {PointerEvent} e */
  function handlePointerDown(e) {
    if (e.button !== 0) return;
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = width;
    dragging = true;
    const handle = /** @type {HTMLElement} */ (e.currentTarget);
    handle.setPointerCapture(e.pointerId);

    /** @param {PointerEvent} move */
    const onMove = (move) => report(startWidth + (startX - move.clientX));
    const onUp = () => {
      dragging = false;
      handle.removeEventListener('pointermove', onMove);
      handle.removeEventListener('pointerup', onUp);
      handle.removeEventListener('pointercancel', onUp);
    };

    handle.addEventListener('pointermove', onMove);
    handle.addEventListener('pointerup', onUp);
    handle.addEventListener('pointercancel', onUp);
  }

  /** @param {KeyboardEvent} e */
  function handleKeydown(e) {
    const step = e.shiftKey ? 64 : 16;
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      report(width + step);
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      report(width - step);
    } else if (e.key === 'Home') {
      e.preventDefault();
      report(max);
    } else if (e.key === 'End') {
      e.preventDefault();
      report(min);
    }
  }
</script>

<!-- A slider, not a plain separator: the width is a value a screen reader can
     read out and the arrow keys can move, and the role stays focusable. -->
<div
  role="slider"
  tabindex="0"
  aria-orientation="vertical"
  aria-label={label}
  aria-valuenow={width}
  aria-valuemin={min}
  aria-valuemax={max}
  title="{label} · drag, or arrow keys"
  onpointerdown={handlePointerDown}
  onkeydown={handleKeydown}
  ondblclick={() => report(min)}
  class="group/resizer absolute inset-y-0 left-0 z-10 flex w-2.5 cursor-col-resize items-center justify-center
    focus-visible:outline-none"
>
  <!-- A line appears under the pointer, so the edge reads as a handle only
       when it is about to be used. -->
  <span
    aria-hidden="true"
    class="h-10 w-0.5 rounded-full transition-colors
      {dragging
      ? 'bg-primary'
      : 'bg-transparent group-hover/resizer:bg-hair group-focus-visible/resizer:bg-primary'}"
  ></span>
</div>
