<script>
  /**
   * @file The `[[` menu, over the Rust editing surface.
   *
   * `DocRefMenu` is the list and is shared with the Milkdown path; this is the
   * trigger, the placing and the keys — the same division as the slash menu,
   * and for the same reason.
   */
  import DocRefMenu from '$lib/components/plugins/DocRefMenu.svelte';
  import {
    docRefItemsFrom,
    filterDocRefItems,
    docRefTrigger,
    docRefHref,
  } from '$lib/editor/doc-ref.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';

  /**
   * @type {{
   *   info: import('$lib/editor/dom.js').SelectionInfo | null,
   *   surface: {
   *     runAsOneEdit: (commands: any[]) => Promise<void>,
   *     focus: () => void,
   *   } | null,
   * }}
   */
  let { info, surface } = $props();

  let trigger = $derived(
    info && info.collapsed ? docRefTrigger(info.blockText, info.offset) : null
  );
  let filtered = $derived(
    trigger ? filterDocRefItems(docRefItemsFrom(fileManager.files), trigger.query) : []
  );

  let selected = $state(0);
  $effect(() => {
    void trigger?.query;
    selected = 0;
  });

  /** Dismissed with Escape: stays shut until the trigger changes. */
  let dismissed = $state(/** @type {string | null} */ (null));
  let showing = $derived(filtered.length > 0 && dismissed !== triggerKey());

  function triggerKey() {
    return trigger ? `${info?.block}:${trigger.start}:${trigger.query}` : null;
  }

  let anchor = $derived(info ? { top: info.rect.bottom + 6, left: info.rect.left } : null);

  /**
   * Replaces `[[query]]` with the writing's title, linked to it.
   *
   * Four commands as one edit: the brackets go, the title goes in, and the link
   * is put over the title — which needs its own selection, since the engine
   * links a range rather than taking a run to insert.
   * @param {import('$lib/editor/doc-ref.js').DocRefItem} item
   */
  async function run(item) {
    if (!trigger || !info) return;
    const block = info.block;
    const text = item.title || item.name;
    const at = (/** @type {number} */ offset) => ({ block, offset });

    await surface?.runAsOneEdit([
      { command: 'setSelection', anchor: at(trigger.start), head: at(trigger.end) },
      { command: 'delete', forward: true },
      { command: 'insertText', text },
      {
        command: 'setSelection',
        anchor: at(trigger.start),
        head: at(trigger.start + text.length),
      },
      { command: 'setLink', url: docRefHref(item.name) },
      // The caret belongs after the link, not inside it, or the next word typed
      // would join the reference.
      {
        command: 'setSelection',
        anchor: at(trigger.start + text.length),
        head: at(trigger.start + text.length),
      },
    ]);
    surface?.focus();
  }

  /** @param {KeyboardEvent} event */
  function onKeydown(event) {
    if (!showing || event.defaultPrevented) return;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      selected = (selected + 1) % filtered.length;
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      selected = (selected - 1 + filtered.length) % filtered.length;
    } else if (event.key === 'Enter' || event.key === 'Tab') {
      const item = filtered[selected];
      if (item) {
        event.preventDefault();
        run(item);
      }
    } else if (event.key === 'Escape') {
      event.preventDefault();
      dismissed = triggerKey();
    }
  }
</script>

<svelte:window onkeydowncapture={onKeydown} />

{#if showing && anchor}
  <div class="fixed z-50" style="top: {anchor.top}px; left: {anchor.left}px">
    <DocRefMenu items={filtered} selectedIndex={selected} onSelect={run} />
  </div>
{/if}
