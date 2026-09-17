<script>
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { STATUS_PRESETS } from '$lib/front-matter.js';

  /** @type {{ fileName: string, isDraft?: boolean }} */
  let { fileName, isDraft = false } = $props();

  const listId = 'scene-status-presets';

  let synopsis = $state('');
  let status = $state('');

  // Follow the open writing, but don't clobber what's being typed.
  let editingSynopsis = false;
  let editingStatus = false;
  /** The writing a field was focused in; an edit never lands on another one. */
  let editingFor = '';
  $effect(() => {
    const meta = editorState.meta;
    if (!editingSynopsis) synopsis = meta.synopsis ?? '';
    if (!editingStatus) status = meta.status ?? '';
  });

  async function commitSynopsis() {
    editingSynopsis = false;
    if (editingFor !== fileName) return;
    const next = synopsis.trim();
    if (next === (editorState.meta.synopsis ?? '')) return;
    await editorState.updateMeta({ synopsis: next || undefined });
  }

  async function commitStatus() {
    editingStatus = false;
    if (editingFor !== fileName) return;
    const next = status.trim();
    if (next === (editorState.meta.status ?? '')) return;
    await editorState.updateMeta({ status: next || undefined });
  }
</script>

<div class="px-4 pb-4 grid gap-3 font-sans">
  <label class="grid gap-1">
    <span class="text-xs text-muted-foreground">Status</span>
    <input
      list={listId}
      bind:value={status}
      placeholder="No status"
      disabled={isDraft}
      onfocus={() => {
        editingStatus = true;
        editingFor = fileName;
      }}
      onchange={commitStatus}
      onblur={commitStatus}
      onkeydown={(e) => {
        if (e.key === 'Enter') e.currentTarget.blur();
      }}
      class="h-8 w-full rounded-md border border-border/60 bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50"
    />
    <datalist id={listId}>
      {#each STATUS_PRESETS as preset (preset)}
        <option value={preset}></option>
      {/each}
    </datalist>
  </label>

  <label class="grid gap-1">
    <span class="text-xs text-muted-foreground">Synopsis</span>
    <textarea
      bind:value={synopsis}
      rows="4"
      placeholder={isDraft ? 'Start writing first' : 'What happens here, in a line or two'}
      disabled={isDraft}
      onfocus={() => {
        editingSynopsis = true;
        editingFor = fileName;
      }}
      onblur={commitSynopsis}
      onkeydown={(e) => {
        if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) e.currentTarget.blur();
      }}
      class="w-full resize-y rounded-md border border-border/60 bg-transparent px-2 py-1.5 font-writer text-sm leading-snug outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50"
    ></textarea>
  </label>
</div>
