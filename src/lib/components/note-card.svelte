<script>
  import { formatTimeAgo } from '$lib/utils.js';
  import { shortcut } from '$lib/keyboard.svelte.js';
  import { cn } from '$lib/utils';
  import { saveNote, deleteNote, revealNote } from '$lib/components/plugins/margin-notes.js';
  import { notesState } from '$lib/runes/notes.svelte.js';

  /**
   * One margin note: what the writer wrote, when, and ways to edit or resolve
   * it. A note whose text has gone from the writing says so, and shows what
   * it was on.
   * @type {{
   *   note: import('$lib/runes/notes.svelte.js').Note,
   *   editing?: boolean,
   *   class?: string,
   * }}
   */
  let { note, editing = false, class: className = '' } = $props();

  let active = $derived(notesState.activeId === note.id);
  let placed = $derived(note.id in notesState.anchors);

  /** @type {HTMLTextAreaElement | null} */
  let field = $state(null);

  /** Set by Escape, so the blur that follows doesn't save what was abandoned. */
  let cancelled = false;

  function save() {
    if (!field || cancelled) return;
    saveNote(note.id, field.value);
  }

  function cancel() {
    cancelled = true;
    // A note that was never written has nothing to go back to.
    if (!note.text) deleteNote(note.id);
    else notesState.editingId = null;
  }

  /** @param {KeyboardEvent} e */
  function onKeydown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      cancel();
    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      save();
    }
  }

  /** Grows the box with what's typed, and puts the caret in it. @param {HTMLTextAreaElement} el */
  function autosize(el) {
    cancelled = false;
    const fit = () => {
      el.style.height = 'auto';
      el.style.height = `${el.scrollHeight}px`;
    };
    fit();
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
    el.addEventListener('input', fit);
    return () => el.removeEventListener('input', fit);
  }
</script>

<article
  class={cn(
    'glass rounded-group px-4.5 py-4 text-sm leading-normal text-foreground transition-shadow',
    (active || editing) && 'ring-2 ring-primary/40',
    className
  )}
  aria-label="Note"
>
  <header class="flex items-baseline justify-between gap-2 text-meta text-metadata">
    <span class="font-medium text-muted-foreground">Note</span>
    {#if note.at}
      <time datetime={note.at}>{formatTimeAgo(note.at)}</time>
    {/if}
  </header>

  {#if editing}
    <label class="sr-only" for="note-{note.id}">Note</label>
    <textarea
      id="note-{note.id}"
      bind:this={field}
      {@attach autosize}
      value={note.text}
      onkeydown={onKeydown}
      onblur={save}
      rows="2"
      placeholder="Write a note to yourself…"
      class="mt-1.5 block w-full resize-none bg-transparent font-writer text-[0.9375rem] leading-relaxed outline-none placeholder:text-placeholder"
    ></textarea>
    <p class="mt-1 text-meta text-metadata">{shortcut('↵')} to save · Esc to cancel</p>
  {:else}
    {#if !placed}
      <p class="mt-1.5 text-meta text-metadata">
        The text this note was on is no longer in the writing: <q class="italic">{note.quote}</q>
      </p>
    {/if}
    <button
      type="button"
      onclick={() => (placed ? revealNote(note.id) : (notesState.editingId = note.id))}
      class="mt-1.5 block w-full text-left font-writer text-[0.9375rem] leading-relaxed whitespace-pre-wrap rounded-sm focus-visible:outline-2 focus-visible:outline-ring"
    >
      {note.text}
    </button>
    <div class="mt-2 flex gap-3 text-meta text-muted-foreground">
      <button type="button" class="hover:text-foreground" onclick={() => (notesState.editingId = note.id)}>Edit</button>
      <button type="button" class="hover:text-foreground" onclick={() => deleteNote(note.id)}>Resolve</button>
    </div>
  {/if}
</article>
