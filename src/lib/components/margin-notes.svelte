<script>
  import NoteCard from '$lib/components/note-card.svelte';
  import { notesState } from '$lib/runes/notes.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { measureNotes } from '$lib/components/plugins/margin-notes.js';

  /**
   * Margin notes beside the page, each level with the line it is on. Cards
   * that would overlap are pushed down in order. Shown only when the window
   * leaves room beside the page; otherwise the scene panel lists the notes.
   */

  /** Card width plus the gap to the page and the window edge, in px. */
  const CARD_WIDTH = 256;
  const ROOM_NEEDED = CARD_WIDTH + 40;
  const GAP = 8;

  /** @type {HTMLElement | null} */
  let root = $state(null);
  let room = $state(0);
  /** @type {Record<string, number>} */
  let tops = $state({});
  /** @type {Record<string, HTMLElement>} */
  const cards = {};

  /** Cards change height as a note is typed or edited; the ones below make way. */
  const cardObserver = new ResizeObserver(() => requestAnimationFrame(layout));

  /** @param {string} id @returns {import('svelte/attachments').Attachment<HTMLElement>} */
  function card(id) {
    return (el) => {
      cards[id] = el;
      cardObserver.observe(el);
      return () => {
        cardObserver.unobserve(el);
        if (cards[id] === el) delete cards[id];
      };
    };
  }

  $effect(() => () => cardObserver.disconnect());

  let visible = $derived(room >= ROOM_NEEDED && !writingState.focusMode);
  let placedNotes = $derived(notesState.list.filter((n) => n.id in notesState.anchors));

  $effect(() => {
    notesState.marginVisible = visible;
    return () => {
      notesState.marginVisible = false;
    };
  });

  // No room for the margin: a new note opens in the scene panel instead.
  $effect(() => {
    if (notesState.editingId && !visible && !appState.ui.isOutlineOpen && !writingState.focusMode) {
      appState.toggleOutline();
    }
  });

  /** The page the notes sit beside. */
  function surface() {
    return /** @type {HTMLElement | null} */ (root?.parentElement ?? null);
  }

  function measureRoom() {
    const page = surface();
    const main = page?.closest('main');
    if (!page || !main) return;
    room = main.getBoundingClientRect().right - page.getBoundingClientRect().right;
  }

  // Room changes with the window, the side panels and the font size.
  $effect(() => {
    const page = surface();
    const main = page?.closest('main');
    if (!page || !main) return;
    const observer = new ResizeObserver(measureRoom);
    observer.observe(main);
    observer.observe(page);
    measureRoom();
    return () => observer.disconnect();
  });

  /** Lines each card up with its text, then pushes overlapping ones down. */
  function layout() {
    const page = surface();
    if (!page || !visible) return;
    /** @type {Record<string, number>} */
    const next = {};
    let floor = -Infinity;
    for (const { id, top } of measureNotes(page)) {
      const y = Math.max(top, floor);
      next[id] = y;
      floor = y + (cards[id]?.offsetHeight ?? 0) + GAP;
    }
    tops = next;
  }

  // Re-lay out after the document moves, a note is added or edited, or the room changes.
  $effect(() => {
    void notesState.layoutVersion;
    void notesState.editingId;
    void placedNotes.length;
    void room;
    if (!visible) return;
    const frame = requestAnimationFrame(layout);
    return () => cancelAnimationFrame(frame);
  });
</script>

<div bind:this={root} class="absolute inset-y-0 left-full" aria-hidden={!visible}>
  {#if visible}
    <aside aria-label="Notes" class="absolute top-0 left-0 ml-6" style="width: {CARD_WIDTH}px">
      {#each placedNotes as note (note.id)}
        <div
          {@attach card(note.id)}
          class="absolute inset-x-0 transition-[top] duration-150 ease-out motion-reduce:transition-none"
          style="top: {tops[note.id] ?? 0}px; visibility: {note.id in tops ? 'visible' : 'hidden'}"
        >
          <NoteCard {note} editing={notesState.editingId === note.id} />
        </div>
      {/each}
    </aside>
    <!-- Notes whose text is gone have no line to sit beside; they wait in the
         scene panel. Pinned so it stays in sight while scrolling. -->
    {#if notesState.detached.length > 0}
      <div class="sticky top-3 z-10 ml-6 pointer-events-none" style="width: {CARD_WIDTH}px">
        <button
          type="button"
          onclick={() => {
            if (!appState.ui.isOutlineOpen) appState.toggleOutline();
          }}
          class="pointer-events-auto px-2.5 py-1 rounded-full text-meta text-muted-foreground bg-popover/80 backdrop-blur border border-border/60 shadow-xs hover:text-foreground transition-colors"
        >
          {notesState.detached.length === 1
            ? '1 note lost its place'
            : `${notesState.detached.length} notes lost their place`}
        </button>
      </div>
    {/if}
  {/if}
</div>
