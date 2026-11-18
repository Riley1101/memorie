<script>
  import { tick } from 'svelte';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { STATUS_PRESETS } from '$lib/front-matter.js';
  import { statusTone } from '$lib/runes/scenes.svelte.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import CheckIcon from '@lucide/svelte/icons/check';

  /** @type {{ fileName: string, isDraft?: boolean }} */
  let { fileName, isDraft = false } = $props();

  let synopsis = $state('');
  let status = $state('');

  /** The custom-status field, open from the menu or by typing one already. */
  let typingCustom = $state(false);
  /** @type {HTMLInputElement | null} */
  let customField = $state(null);

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

  // A writing opened with a status of its own closes the custom field again.
  $effect(() => {
    fileName;
    typingCustom = false;
  });

  let tone = $derived(statusTone(status));

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

  /** Sets a status straight from the menu. @param {string} next */
  async function choose(next) {
    typingCustom = false;
    status = next;
    if (next === (editorState.meta.status ?? '')) return;
    await editorState.updateMeta({ status: next || undefined });
  }

  async function startCustom() {
    typingCustom = true;
    await tick();
    customField?.focus();
    customField?.select();
  }
</script>

<div class="px-4 pb-4 grid gap-3 font-sans">
  <div class="grid gap-1">
    <span class="text-xs text-muted-foreground" id="scene-status-label">Status</span>
    <!-- The status carries its own colour here, the same one the writing shows
         as a dot in the binder; the menu names the whole set rather than
         hiding it behind a text field's suggestions. -->
    <DropdownMenu.Root>
      <DropdownMenu.Trigger disabled={isDraft}>
        {#snippet child({ props })}
          <button
            {...props}
            type="button"
            aria-labelledby="scene-status-label"
            disabled={isDraft}
            class="flex items-center gap-2 h-8 w-full pl-2.5 pr-2 rounded-full text-sm font-medium text-left
              ring-1 ring-inset transition-colors disabled:opacity-50
              focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50
              {status ? `${tone.chip} ring-current/25` : 'bg-fill text-muted-foreground ring-hair'}"
          >
            <span
              class="size-2 rounded-full shrink-0 {status ? tone.dot : 'bg-muted-foreground/40'}"
            ></span>
            <span class="flex-1 min-w-0 truncate">{status || 'No status'}</span>
            <ChevronDownIcon strokeWidth={1.8} class="size-3.5 shrink-0 opacity-70" />
          </button>
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content class="w-56" align="start" portalProps={{}}>
        {#each STATUS_PRESETS as preset (preset)}
          {@const presetTone = statusTone(preset)}
          <DropdownMenu.Item class="" inset={false} onclick={() => choose(preset)}>
            <span class="size-2 rounded-full shrink-0 {presetTone.dot}"></span>
            <span class="flex-1 min-w-0 truncate">{preset}</span>
            {#if status === preset}
              <CheckIcon strokeWidth={2} class="size-3.5 text-muted-foreground" />
            {/if}
          </DropdownMenu.Item>
        {/each}
        <DropdownMenu.Separator class="" />
        <DropdownMenu.Item class="" inset={false} onclick={startCustom}>Custom…</DropdownMenu.Item>
        <DropdownMenu.Item class="" inset={false} disabled={!status} onclick={() => choose('')}>
          Clear status
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>

    {#if typingCustom}
      <!-- A status of the writer's own: the same field as before, now only
           where it is asked for. -->
      <input
        bind:this={customField}
        bind:value={status}
        placeholder="Name this status"
        aria-label="Custom status"
        disabled={isDraft}
        onfocus={() => {
          editingStatus = true;
          editingFor = fileName;
        }}
        onblur={() => {
          commitStatus();
          typingCustom = false;
        }}
        onkeydown={(e) => {
          if (e.key === 'Enter') e.currentTarget.blur();
          if (e.key === 'Escape') {
            status = editorState.meta.status ?? '';
            editingStatus = false;
            typingCustom = false;
          }
        }}
        class="raised h-8 w-full rounded-[6px] px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50"
      />
    {/if}
  </div>

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
      class="raised w-full resize-y rounded-[6px] px-2 py-1.5 font-writer text-sm leading-snug outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50"
    ></textarea>
  </label>
</div>
