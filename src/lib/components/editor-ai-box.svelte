<script>
  /**
   * @file The AI box, floated over the Rust editing surface.
   *
   * `GrammarBox` is the panel itself and is shared with the Milkdown path; this
   * is only the part Milkdown got from being a ProseMirror plugin — a place to
   * stand, and a way to act on the selection. The surface reports where the
   * selection is (`onselection`), this positions the box there and keeps it
   * there while the page scrolls.
   *
   * Why not a widget inside the document, which is what the Milkdown version
   * is: the box is `position: fixed` and repositions on scroll in that version
   * too, so the decoration was only ever a mounting hook. Keeping it out of the
   * document means the caret mapping never has to know it exists.
   */
  import GrammarBox from '$lib/components/plugins/GrammarBox.svelte';

  /**
   * @type {{
   *   info: import('$lib/editor/dom.js').SelectionInfo | null,
   *   surface: { toggleMark: (mark: 'bold' | 'italic' | 'strike' | 'code') => void,
   *              replaceBlock: (block: number, markdown: string) => void,
   *              selectionInfo: () => any } | null,
   * }}
   * `info` is the last selection the surface reported; null hides the box.
   */
  let { info: reported, surface } = $props();

  /**
   * The box acts on a selection, so a bare caret is not its business — the
   * surface reports those too, for the panels that answer them.
   */
  let info = $derived(reported && !reported.collapsed ? reported : null);

  /**
   * Where the box sits: under the selection, nudged in from the left edge.
   *
   * Recomputed from `info.rect` rather than measured, so a scroll only has to
   * ask the surface for a fresh rect. The box is not flipped above the line
   * when it would fall off the bottom — it scrolls with the page, and the
   * Milkdown version never did either.
   */
  let anchor = $derived.by(() =>
    info ? { top: info.rect.bottom + 8, left: info.rect.left } : null
  );
  /** Where a scroll has moved it to since, which wins while it is set. */
  let scrolled = $state(/** @type {{ top: number, left: number } | null} */ (null));
  let placed = $derived(scrolled ?? anchor);

  $effect(() => {
    if (!info) return;
    // A new selection starts from its own rect, not from where the last one had
    // been scrolled to.
    scrolled = null;
    // A scroll moves the text out from under the box, so the rect is asked for
    // again rather than offset: the surface knows where the selection is now,
    // and a wrapped line can move sideways as well as up.
    const reanchor = () => {
      const fresh = surface?.selectionInfo();
      if (fresh) scrolled = { top: fresh.rect.bottom + 8, left: fresh.rect.left };
    };
    window.addEventListener('scroll', reanchor, { capture: true, passive: true });
    window.addEventListener('resize', reanchor);
    return () => {
      window.removeEventListener('scroll', reanchor, { capture: true });
      window.removeEventListener('resize', reanchor);
    };
  });
</script>

{#if info && placed}
  <!-- `mousedown` is swallowed so clicking the box doesn't collapse the
       selection it is acting on — the same job `stopEvent` does for the
       Milkdown widget. -->
  <div
    class="fixed z-50"
    style="top: {placed.top}px; left: {placed.left}px"
    onmousedown={(event) => event.preventDefault()}
    role="toolbar"
    tabindex="-1"
    aria-label="Writing suggestions"
  >
    <GrammarBox
      sequence={0}
      originalText={info.blockText}
      onFix={(suggestion) => surface?.replaceBlock(info.block, suggestion)}
      onMark={(mark) => surface?.toggleMark(mark)}
      selectedText={() => surface?.selectionInfo()?.text ?? info.text}
    />
  </div>
{/if}
