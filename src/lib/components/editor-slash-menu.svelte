<script>
  /**
   * @file The slash menu, over the Rust editing surface.
   *
   * `SlashMenu` is the list and is shared with the Milkdown path; this is the
   * part Milkdown got from `SlashProvider` — noticing the trigger, placing the
   * list, and the arrow keys.
   *
   * The trigger is read out of the block's text rather than watched for as a
   * keystroke, so it is right however the `/` arrived: typed, pasted, or left
   * behind when the writer deleted the word after it.
   */
  import SlashMenu from '$lib/components/plugins/SlashMenu.svelte';
  import { slashItems, filterSlashItems, slashTrigger } from '$lib/editor/slash-items.js';
  import { MOD_KEY } from '$lib/keyboard.svelte.js';

  /**
   * @type {{
   *   info: import('$lib/editor/dom.js').SelectionInfo | null,
   *   surface: {
   *     runAsOneEdit: (commands: any[]) => Promise<void>,
   *     selectionInfo: () => import('$lib/editor/dom.js').SelectionInfo | null,
   *     focus: () => void,
   *   } | null,
   *   onlink?: () => Promise<string | null> | string | null,
   * }}
   */
  let { info, surface, onlink } = $props();

  const items = slashItems(MOD_KEY);

  /** The trigger under the caret, if there is one. A selection is not one. */
  let trigger = $derived(info && info.collapsed ? slashTrigger(info.blockText, info.offset) : null);
  let filtered = $derived(trigger ? filterSlashItems(items, trigger.query) : []);
  let open = $derived(filtered.length > 0);

  let selected = $state(0);
  $effect(() => {
    // A new query starts at the top of its own list.
    void trigger?.query;
    selected = 0;
  });

  /** Dismissed with Escape: stays shut until the trigger changes. */
  let dismissed = $state(/** @type {string | null} */ (null));
  let showing = $derived(open && dismissed !== triggerKey());

  function triggerKey() {
    return trigger ? `${info?.block}:${trigger.start}:${trigger.query}` : null;
  }

  let anchor = $derived(info ? { top: info.rect.bottom + 6, left: info.rect.left } : null);

  /**
   * Runs an item: the `/query` goes first, then the command, as one edit — so
   * one undo takes both the typing and what it turned into.
   * @param {import('$lib/editor/slash-items.js').SlashItem} item
   */
  async function run(item) {
    if (!trigger || !info) return;
    const block = info.block;
    const { start } = trigger;
    const end = info.offset;

    /** @type {any[]} */
    const commands = [
      {
        command: 'setSelection',
        anchor: { block, offset: start },
        head: { block, offset: end },
      },
      { command: 'delete', forward: true },
    ];

    if (item.asks === 'link') {
      const url = await onlink?.();
      if (!url) {
        // Nothing to link to, so the `/link` the writer typed stays put rather
        // than being eaten by a command that did not run.
        dismissed = triggerKey();
        return;
      }
      commands.push({ command: 'setLink', url });
    } else {
      commands.push(...(item.commands ?? []));
    }

    await surface?.runAsOneEdit(commands);
    surface?.focus();
  }

  /**
   * The menu owns the arrow keys while it is open. Captured on the window
   * rather than taken from the surface, because the surface hands its keys to
   * the engine and a menu that is only sometimes in charge of ↑ and ↓ does not
   * belong in that path.
   * @param {KeyboardEvent} event
   */
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
    <SlashMenu items={filtered} selectedIndex={selected} onSelect={run} />
  </div>
{/if}
