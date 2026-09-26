<script>
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import FileTextIcon from '@lucide/svelte/icons/file-text';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import UnlinkIcon from '@lucide/svelte/icons/unlink';
  import { untrack } from 'svelte';
  import { MOD_KEY } from '$lib/keyboard.svelte.js';

  /**
   * @typedef {Object} Props
   * @property {'hidden' | 'view' | 'edit'} mode
   * @property {string} href - The link's current href; the starting value when editing.
   * @property {string} label - What the link points at, for display: a URL or a writing's title.
   * @property {boolean} isDocRef
   * @property {boolean} needsText - Editing a new link at an empty caret, so ask for its text too.
   * @property {() => void} onOpen
   * @property {() => void} onEdit
   * @property {() => void} onRemove
   * @property {(href: string, text: string) => void} onApply
   * @property {() => void} onCancel - Esc: stop editing and return to the editor.
   * @property {() => void} onDismiss - Focus left for elsewhere: stop editing, leave focus be.
   */

  /** @type {Props} */
  let { mode, href, label, isDocRef, needsText, onOpen, onEdit, onRemove, onApply, onCancel, onDismiss } =
    $props();

  let hrefInput = $state('');
  let textInput = $state('');

  /**
   * Mounted each time editing starts: fills the fields and focuses the first one.
   * @param {HTMLElement} form
   */
  function startForm(form) {
    untrack(() => {
      hrefInput = href;
      textInput = '';
    });
    const first = form.querySelector('input');
    first?.focus();
    first?.select();
  }

  /** @param {KeyboardEvent} e */
  function onKeydown(e) {
    if (e.key === 'Enter') {
      e.preventDefault();
      onApply(hrefInput, textInput);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onCancel();
    }
  }

  /** @param {FocusEvent} e */
  function onFocusout(e) {
    const next = /** @type {Node | null} */ (e.relatedTarget);
    if (next && /** @type {HTMLElement} */ (e.currentTarget).contains(next)) return;
    onDismiss();
  }

  const button =
    'size-7 shrink-0 flex items-center justify-center rounded-sm text-muted-foreground hover:bg-muted hover:text-foreground transition-colors';
</script>

{#if mode === 'view'}
  <div
    class="flex items-center gap-1 max-w-sm bg-popover border border-border rounded-lg shadow-lg p-1 not-prose font-sans"
    role="toolbar"
    aria-label="Link"
  >
    <button
      type="button"
      class="min-w-0 flex items-center gap-2 px-2 h-7 rounded-sm text-xs text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
      title={isDocRef ? 'Open writing' : `Open link (${MOD_KEY}-click)`}
      onmousedown={(e) => e.preventDefault()}
      onclick={onOpen}
    >
      {#if isDocRef}
        <FileTextIcon class="size-3.5 shrink-0" />
      {:else}
        <ExternalLinkIcon class="size-3.5 shrink-0" />
      {/if}
      <span class="truncate">{label || 'Empty link'}</span>
    </button>
    <button
      type="button"
      class={button}
      title="Edit link"
      aria-label="Edit link"
      onmousedown={(e) => e.preventDefault()}
      onclick={onEdit}
    >
      <PencilIcon class="size-3.5" />
    </button>
    <button
      type="button"
      class={button}
      title="Remove link"
      aria-label="Remove link"
      onmousedown={(e) => e.preventDefault()}
      onclick={onRemove}
    >
      <UnlinkIcon class="size-3.5" />
    </button>
  </div>
{:else if mode === 'edit'}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="w-80 flex flex-col gap-1 bg-popover border border-border rounded-lg shadow-lg p-1.5 not-prose font-sans"
    onkeydown={onKeydown}
    onfocusout={onFocusout}
    {@attach startForm}
  >
    {#if needsText}
      <input
        bind:value={textInput}
        class="h-8 px-2 rounded-sm bg-muted/40 text-sm text-foreground outline-none placeholder:text-muted-foreground"
        placeholder="Text"
        aria-label="Link text"
        spellcheck="false"
        autocomplete="off"
      />
    {/if}
    <input
      bind:value={hrefInput}
      class="h-8 px-2 rounded-sm bg-muted/40 text-sm text-foreground outline-none placeholder:text-muted-foreground"
      placeholder="Paste or type a link"
      aria-label="Link address"
      spellcheck="false"
      autocomplete="off"
    />
    <p class="px-2 pb-0.5 text-[0.6875rem] text-muted-foreground">
      Enter to apply{needsText ? '' : ', empty to remove'} · Esc to cancel
    </p>
  </div>
{/if}
