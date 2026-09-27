<script>
  /**
   * CodexSheet - edits one codex entry: its name, kind, aliases, fields,
   * relationships and notes. The parent keys it by entry, so the draft starts
   * over whenever a different entry is opened.
   */
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import XIcon from '@lucide/svelte/icons/x';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { toast } from '$lib/toast.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { KINDS, codexManager, kindInfo } from '$lib/runes/codex.svelte.js';
  import { isMod } from '$lib/keyboard.svelte.js';

  /**
   * @typedef {import('$lib/runes/codex.svelte.js').Entity} Entity
   * @typedef {{ name: string, label: string, count: number }} Appearance
   */

  /**
   * @type {{
   *   entity: Entity,
   *   binders: string[],
   *   names: string[],
   *   appearances?: Appearance[] | null,
   *   onsaved?: (entity: Entity, previous: Entity) => void,
   *   ondeleted?: () => void,
   * }}
   */
  let { entity, binders, names, appearances = null, onsaved, ondeleted } = $props();

  /**
   * Adds the kind's usual fields that the sheet doesn't have yet. Empty fields
   * aren't written to the file, so they'd otherwise vanish after a save.
   * @param {import('$lib/runes/codex.svelte.js').Field[]} fields
   * @param {import('$lib/runes/codex.svelte.js').EntityKind} kind
   */
  function withTemplate(fields, kind) {
    const have = new Set(fields.map((f) => f.key.trim().toLowerCase()));
    const missing = kindInfo(kind).fields.filter((key) => !have.has(key));
    return [...fields, ...missing.map((key) => ({ key, value: '' }))];
  }

  /** @param {Entity} e */
  function toDraft(e) {
    const copy = $state.snapshot(e);
    return { ...copy, fields: withTemplate(copy.fields, copy.kind), aliasText: copy.aliases.join(', ') };
  }

  /**
   * Switches kind, swapping the old kind's empty template fields for the new one's.
   * @param {import('$lib/runes/codex.svelte.js').EntityKind} kind
   */
  function setKind(kind) {
    if (kind === draft.kind) return;
    const oldTemplate = new Set(kindInfo(draft.kind).fields);
    const kept = draft.fields.filter((f) => f.value.trim() || !oldTemplate.has(f.key.trim().toLowerCase()));
    draft.kind = kind;
    draft.fields = withTemplate(kept, kind);
  }

  // Seeded once: the parent remounts this component for another entry.
  // svelte-ignore state_referenced_locally
  let draft = $state(toDraft(entity));
  let saving = $state(false);
  let confirmDelete = $state(false);

  let isNew = $derived(!entity.path);

  /** Suggestions from the writings, waiting to be used or dismissed. @type {import('$lib/runes/codex.svelte.js').SheetDraft | null} */
  let suggestions = $state(null);
  let suggesting = $state(false);

  let aiEnabled = $derived(Boolean(configManager.config?.ai_enabled));
  /** Fields with a name but nothing in them yet. */
  let emptyKeys = $derived(draft.fields.filter((f) => f.key.trim() && !f.value.trim()).map((f) => f.key.trim()));
  let canSuggest = $derived(!isNew && (emptyKeys.length > 0 || !draft.summary.trim()));

  async function suggest() {
    suggesting = true;
    try {
      const found = await codexManager.draft(entity.path, emptyKeys);
      const count = Object.keys(found.fields).length + (found.summary ? 1 : 0);
      if (!count) {
        toast.info('Nothing to suggest', 'The writings that mention this entry don’t say more yet.');
        suggestions = null;
      } else {
        suggestions = found;
      }
    } catch (err) {
      toast.error('Could not draft suggestions', err);
    } finally {
      suggesting = false;
    }
  }

  /** @param {string} key */
  function suggestionFor(key) {
    if (!suggestions) return '';
    const lower = key.trim().toLowerCase();
    const match = Object.entries(suggestions.fields).find(([k]) => k.toLowerCase() === lower);
    return match?.[1] ?? '';
  }

  /** @param {string} key */
  function dropSuggestion(key) {
    if (!suggestions) return;
    const lower = key.trim().toLowerCase();
    suggestions.fields = Object.fromEntries(Object.entries(suggestions.fields).filter(([k]) => k.toLowerCase() !== lower));
  }

  function useAll() {
    if (!suggestions) return;
    if (suggestions.summary && !draft.summary.trim()) draft.summary = suggestions.summary;
    for (const field of draft.fields) {
      const value = suggestionFor(field.key);
      if (value && !field.value.trim()) field.value = value;
    }
    suggestions = null;
  }

  /** The draft as it would be saved. */
  function normalized() {
    const { aliasText, ...rest } = $state.snapshot(draft);
    return {
      ...rest,
      name: rest.name.trim(),
      aliases: aliasText
        .split(',')
        .map((a) => a.trim())
        .filter(Boolean),
      summary: rest.summary.trim(),
      notes: rest.notes.trim(),
      fields: rest.fields
        .map((f) => ({ key: f.key.trim(), value: f.value.trim() }))
        .filter((f) => f.key && f.value),
      relationships: rest.relationships
        .map((r) => ({ to: r.to.trim(), kind: r.kind.trim() }))
        .filter((r) => r.to),
    };
  }

  /** @param {Entity} e */
  function comparable(e) {
    const s = $state.snapshot(e);
    return JSON.stringify({
      ...s,
      path: '',
      fields: s.fields
        .map((f) => ({ key: f.key.trim(), value: f.value.trim() }))
        .filter((f) => f.key && f.value),
      relationships: s.relationships
        .map((r) => ({ to: r.to.trim(), kind: r.kind.trim() }))
        .filter((r) => r.to),
      summary: s.summary.trim(),
      notes: s.notes.trim(),
    });
  }

  let dirty = $derived(isNew || comparable(normalized()) !== comparable(entity));
  let canSave = $derived(draft.name.trim().length > 0);

  /** A new sheet with nothing typed into it: fine to drop. */
  let blank = $derived.by(() => {
    if (!isNew) return false;
    const n = normalized();
    return !n.name && !n.aliases.length && !n.summary && !n.notes && !n.fields.length && !n.relationships.length;
  });

  /** Whether leaving now would lose something. */
  export function hasUnsavedChanges() {
    return dirty && !blank;
  }

  /**
   * Saves the draft if it changed. The page calls this before leaving the sheet.
   * @returns {Promise<boolean>} true when it's safe to leave: saved, unchanged,
   *   or a new sheet with nothing in it. False keeps the writer here.
   */
  export async function save() {
    if (!dirty || blank) return true;
    if (!canSave) {
      toast.info('Give this entry a name', 'It needs one before it can be saved.');
      return false;
    }
    if (saving) return false;
    saving = true;
    try {
      const saved = await codexManager.save(normalized(), entity.path || null);
      if (!saved) return false;
      onsaved?.(saved, entity);
      return true;
    } finally {
      saving = false;
    }
  }

  async function remove() {
    confirmDelete = false;
    if (await codexManager.remove(entity)) ondeleted?.();
  }

  function addField() {
    draft.fields.push({ key: '', value: '' });
  }

  function addRelationship() {
    draft.relationships.push({ to: '', kind: '' });
  }

  /** @param {KeyboardEvent} e */
  function onKeydown(e) {
    if (e.key.toLowerCase() === 's' && isMod(e)) {
      e.preventDefault();
      save();
    }
  }

  const labelClass = 'text-xs text-muted-foreground';
  const inputClass =
    'h-8 w-full rounded-md border border-border/60 bg-transparent px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50';
  const selectClass =
    'h-8 w-full rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50';
  const areaClass =
    'w-full rounded-md border border-border/60 bg-transparent px-2 py-1.5 text-sm outline-none field-sizing-content focus-visible:ring-2 focus-visible:ring-ring/50';
  const sectionClass = 'text-[0.6875rem] font-mono uppercase tracking-wider text-metadata';
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form
  class="flex flex-col gap-6 font-sans pb-10"
  onsubmit={(e) => {
    e.preventDefault();
    save();
  }}
  onkeydown={onKeydown}
>
  <div class="flex items-start gap-3">
    <input
      bind:value={draft.name}
      placeholder="Name"
      aria-label="Name"
      autofocus={isNew}
      class="flex-1 min-w-0 bg-transparent font-writer text-3xl text-heading-foreground outline-none placeholder:text-muted-foreground/40"
    />
    <div class="flex items-center gap-1 shrink-0 pt-1">
      {#if !isNew}
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-destructive rounded-full"
          onclick={() => (confirmDelete = true)}
          aria-label="Delete entry"
          disabled={false}
        >
          <Trash2Icon strokeWidth={1.5} class="size-3.5" />
        </Button>
      {/if}
      <Button type="submit" size="sm" disabled={!dirty || !canSave || saving} class="">
        {saving ? 'Saving…' : dirty ? 'Save' : 'Saved'}
      </Button>
    </div>
  </div>

  <div class="grid gap-3 sm:grid-cols-2">
    <fieldset class="grid gap-1">
      <legend class="{labelClass} mb-1">Kind</legend>
      <div class="flex flex-wrap gap-1">
        {#each KINDS as kind (kind.id)}
          <button
            type="button"
            onclick={() => setKind(kind.id)}
            aria-pressed={draft.kind === kind.id}
            class="h-7 px-2.5 rounded-md border text-xs transition-colors
              {draft.kind === kind.id
              ? 'border-primary/40 bg-primary/10 text-primary'
              : 'border-border/60 text-muted-foreground hover:text-foreground'}"
          >
            {kind.label}
          </button>
        {/each}
      </div>
    </fieldset>
    <label class="grid gap-1">
      <span class={labelClass}>Belongs to</span>
      <select bind:value={draft.scope} class={selectClass}>
        <option value={null}>Shared by every binder</option>
        {#each binders as binder (binder)}
          <option value={binder}>{binder}</option>
        {/each}
      </select>
    </label>
  </div>

  <label class="grid gap-1">
    <span class={labelClass}>Also known as <span class="opacity-60">(comma separated)</span></span>
    <input bind:value={draft.aliasText} placeholder="Ari, the Ashborn girl" class={inputClass} />
  </label>

  {#if aiEnabled && !isNew}
    <div class="flex items-center gap-2 -mb-2">
      <Button
        type="button"
        variant="outline"
        size="sm"
        class="gap-1.5"
        onclick={suggest}
        disabled={!canSuggest || suggesting}
        title={canSuggest ? 'Fill empty fields from the writings that mention this entry' : 'Every field already has something in it'}
      >
        <SparklesIcon strokeWidth={1.5} class="size-3 {suggesting ? 'animate-pulse' : ''}" />
        {suggesting ? 'Reading your writings…' : 'Suggest from writings'}
      </Button>
      {#if suggestions}
        <Button type="button" variant="ghost" size="sm" onclick={useAll} disabled={false} class="">Use all</Button>
        <Button type="button" variant="ghost" size="sm" onclick={() => (suggestions = null)} disabled={false} class="text-muted-foreground">
          Dismiss
        </Button>
      {/if}
    </div>
  {/if}

  {#snippet suggestion(/** @type {string} */ value, /** @type {() => void} */ use, /** @type {() => void} */ drop)}
    <div class="flex items-start gap-2 rounded-md border border-dashed border-primary/30 bg-primary/5 px-2 py-1.5 text-sm">
      <SparklesIcon strokeWidth={1.5} class="size-3 mt-1 shrink-0 text-primary/60" />
      <span class="flex-1 min-w-0">{value}</span>
      <button type="button" onclick={use} class="text-xs text-primary hover:underline shrink-0">Use</button>
      <button type="button" onclick={drop} aria-label="Dismiss suggestion" class="text-muted-foreground/60 hover:text-foreground shrink-0">
        <XIcon strokeWidth={1.5} class="size-3" />
      </button>
    </div>
  {/snippet}

  <label class="grid gap-1">
    <span class={labelClass}>Summary <span class="opacity-60">(the assistant always sees this)</span></span>
    <textarea bind:value={draft.summary} rows="2" placeholder="One or two lines." class={areaClass}></textarea>
  </label>
  {#if suggestions?.summary && !draft.summary.trim()}
    {@render suggestion(
      suggestions.summary,
      () => {
        if (suggestions) draft.summary = suggestions.summary;
      },
      () => {
        if (suggestions) suggestions.summary = '';
      }
    )}
  {/if}

  <section class="grid gap-2">
    <h3 class={sectionClass}>Details</h3>
    {#each draft.fields as field, i (i)}
      {@const suggested = field.value.trim() ? '' : suggestionFor(field.key)}
      <div class="grid grid-cols-[8rem_1fr_auto] gap-2 items-start">
        <input bind:value={field.key} placeholder="Field" aria-label="Field name" class="{inputClass} text-muted-foreground" />
        <textarea bind:value={field.value} rows="1" aria-label={field.key || 'Field value'} class={areaClass}></textarea>
        <button
          type="button"
          onclick={() => draft.fields.splice(i, 1)}
          aria-label="Remove {field.key || 'field'}"
          class="size-8 flex items-center justify-center rounded-md text-muted-foreground/60 hover:text-foreground"
        >
          <XIcon strokeWidth={1.5} class="size-3.5" />
        </button>
      </div>
      {#if suggested}
        <div class="sm:pl-[8.5rem] -mt-1">
          {@render suggestion(
            suggested,
            () => {
              field.value = suggested;
              dropSuggestion(field.key);
            },
            () => dropSuggestion(field.key)
          )}
        </div>
      {/if}
    {/each}
    <button type="button" onclick={addField} class="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground w-fit">
      <PlusIcon strokeWidth={1.5} class="size-3" /> Add field
    </button>
  </section>

  <section class="grid gap-2">
    <h3 class={sectionClass}>Relationships</h3>
    <datalist id="codex-entry-names">
      {#each names as name (name)}
        <option value={name}></option>
      {/each}
    </datalist>
    {#each draft.relationships as rel, i (i)}
      <div class="grid grid-cols-[1fr_1fr_auto] gap-2 items-center">
        <input bind:value={rel.to} list="codex-entry-names" placeholder="Who or what" aria-label="Related to" class={inputClass} />
        <input bind:value={rel.kind} placeholder="rival, mentor, born in…" aria-label="Relationship" class={inputClass} />
        <button
          type="button"
          onclick={() => draft.relationships.splice(i, 1)}
          aria-label="Remove relationship"
          class="size-8 flex items-center justify-center rounded-md text-muted-foreground/60 hover:text-foreground"
        >
          <XIcon strokeWidth={1.5} class="size-3.5" />
        </button>
      </div>
    {/each}
    <button type="button" onclick={addRelationship} class="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground w-fit">
      <PlusIcon strokeWidth={1.5} class="size-3" /> Add relationship
    </button>
  </section>

  <label class="grid gap-1">
    <span class={sectionClass}>Notes</span>
    <textarea bind:value={draft.notes} rows="6" placeholder="Anything else worth remembering." class="{areaClass} min-h-32"></textarea>
  </label>

  <section class="grid gap-3">
    <h3 class={sectionClass}>Assistant</h3>
    <label class="grid gap-1 max-w-sm">
      <span class={labelClass}>Find in writing</span>
      <select bind:value={draft.matchMode} class={selectClass}>
        <option value="case-sensitive">Exact case, so “Will” but not “will”</option>
        <option value="insensitive">Any case</option>
        <option value="off">Never, only when pinned or related</option>
      </select>
    </label>
    <label class="flex items-center gap-2 text-sm">
      <input type="checkbox" bind:checked={draft.pinned} class="accent-primary" />
      Always give this sheet to the assistant{draft.scope ? ` in ${draft.scope}` : ''}
    </label>
  </section>

  {#if appearances && !isNew}
    <section class="grid gap-2">
      <h3 class={sectionClass}>
        Appears in {appearances.length} writing{appearances.length === 1 ? '' : 's'}
      </h3>
      {#if appearances.length}
        <div class="flex flex-wrap gap-1.5">
          {#each appearances as a (a.name)}
            <button
              type="button"
              onclick={() => goto(resolve(`/${encodeURIComponent(a.name)}`))}
              title={a.name}
              class="rounded-full border border-border bg-muted/30 px-2.5 py-1 text-[0.75rem] text-muted-foreground hover:border-primary/40 hover:bg-primary/10 hover:text-primary transition-colors"
            >
              {a.label} <span class="opacity-50 tabular-nums">×{a.count}</span>
            </button>
          {/each}
        </div>
      {:else}
        <p class="text-sm text-muted-foreground/70">Not mentioned by name or alias in any writing yet.</p>
      {/if}
    </section>
  {/if}
</form>

<Dialog.Root bind:open={confirmDelete}>
  <Dialog.Content class="sm:max-w-[400px]" portalProps={{}}>
    <Dialog.Header class="">
      <Dialog.Title class="text-xl font-normal font-writer">Delete “{entity.name}”?</Dialog.Title>
      <Dialog.Description class="text-base text-muted-foreground/80 pt-2">
        This deletes the sheet's file. Your writings aren't changed.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="mt-6 flex gap-2">
      <Button variant="ghost" onclick={() => (confirmDelete = false)} class="flex-1" disabled={false}>Cancel</Button>
      <Button variant="destructive" onclick={remove} class="flex-1" disabled={false}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
