<script>
  /**
   * EditorOutline - table of contents built from the live editor's headings.
   * Click a heading to jump to it; the one currently in view is highlighted.
   */
  import { onMount } from 'svelte';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { editorViewCtx } from '@milkdown/kit/core';
  import { TextSelection } from '@milkdown/kit/prose/state';

  let headings = $derived(editorState.headings);
  let activeIndex = $state(-1);

  // Indent relative to the shallowest heading, so a doc without an H1 isn't all pushed right.
  let minLevel = $derived(headings.length ? Math.min(...headings.map((h) => h.level)) : 1);

  /** @param {{ pos: number }} heading */
  function jumpTo(heading) {
    editorState.editor?.action((ctx) => {
      const view = ctx.get(editorViewCtx);
      const node = view.state.doc.nodeAt(heading.pos);
      if (!node) return;
      const end = heading.pos + node.nodeSize - 1;
      view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, end)));
      view.focus();
      /** @type {HTMLElement | null} */ (view.nodeDOM(heading.pos))?.scrollIntoView({
        behavior: 'smooth',
        block: 'start',
      });
    });
  }

  /** Last heading whose top has scrolled past the upper part of the viewport. */
  function updateActive() {
    if (!headings.length) {
      activeIndex = -1;
      return;
    }
    editorState.editor?.action((ctx) => {
      const view = ctx.get(editorViewCtx);
      const threshold = window.innerHeight * 0.25;
      let next = 0;
      for (let i = 0; i < headings.length; i++) {
        const dom = /** @type {HTMLElement | null} */ (view.nodeDOM(headings[i].pos));
        if (!dom) continue;
        if (dom.getBoundingClientRect().top <= threshold) next = i;
        else break;
      }
      activeIndex = next;
    });
  }

  $effect(() => {
    void headings;
    updateActive();
  });

  onMount(() => {
    let frame = 0;
    const onScroll = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(updateActive);
    };
    // Capture: the writing scrolls inside a ScrollArea viewport, not the window.
    document.addEventListener('scroll', onScroll, { capture: true, passive: true });
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener('scroll', onScroll, { capture: true });
    };
  });
</script>

<div class="px-3 pb-4">
  {#if headings.length > 0}
    <nav aria-label="Table of contents" class="space-y-0.5">
      <!-- Keyed by index: positions shift on every edit above a heading. -->
      {#each headings as heading, i (i)}
        <button
          onclick={() => jumpTo(heading)}
          title={heading.text}
          aria-current={i === activeIndex ? 'location' : undefined}
          class="block w-full truncate rounded px-2 py-1 text-left text-[0.8125rem] transition-colors hover:bg-foreground/5 hover:text-foreground
            {i === activeIndex ? 'text-foreground' : 'text-muted-foreground/70'}"
          style="padding-left: {0.5 + (heading.level - minLevel) * 0.75}rem"
        >
          {heading.text}
        </button>
      {/each}
    </nav>
  {:else}
    <p class="px-2 text-sm text-muted-foreground">No headings yet</p>
  {/if}
</div>
