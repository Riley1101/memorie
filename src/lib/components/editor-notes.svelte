<script>
  import NoteCard from '$lib/components/note-card.svelte';
  import { notesState } from '$lib/runes/notes.svelte.js';
  import { shortcut } from '$lib/keyboard.svelte.js';

  /**
   * The writing's notes in the scene panel: in the order they appear in the
   * text, then any whose text has gone. This is where notes live when the
   * window has no room for them in the margin, and the one place a note that
   * lost its text can still be read and resolved.
   */

  let ordered = $derived(
    [...notesState.list].sort((a, b) => {
      const pa = notesState.anchors[a.id]?.from ?? Infinity;
      const pb = notesState.anchors[b.id]?.from ?? Infinity;
      return pa - pb;
    })
  );

  // With the margin showing, notes are written there; the panel just lists them.
  let shown = $derived(notesState.marginVisible ? notesState.detached : ordered);
</script>

<div class="px-3 pb-4">
  {#if shown.length > 0}
    <ul class="flex flex-col gap-2">
      {#each shown as note (note.id)}
        <li>
          <NoteCard
            {note}
            editing={notesState.editingId === note.id && (!notesState.marginVisible || !(note.id in notesState.anchors))}
          />
        </li>
      {/each}
    </ul>
  {:else if notesState.list.length > 0}
    <p class="px-1 text-xs text-muted-foreground">Your notes are in the margin beside the text.</p>
  {:else}
    <p class="px-1 text-xs text-muted-foreground">
      Select some text and press {shortcut('alt', 'M')} to leave yourself a note.
    </p>
  {/if}
</div>
