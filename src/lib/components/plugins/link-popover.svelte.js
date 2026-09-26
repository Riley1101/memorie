import { $prose as prose } from '@milkdown/kit/utils';
import { TooltipProvider } from '@milkdown/kit/plugin/tooltip';
import { posToDOMRect } from '@milkdown/kit/prose';
import { Plugin, TextSelection } from '@milkdown/kit/prose/state';
import { mount, unmount } from 'svelte';
import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { openUrl } from '@tauri-apps/plugin-opener';
import { baseOf } from '$lib/runes/fs.svelte.js';
import { isMod } from '$lib/keyboard.svelte.js';
import LinkPopover from './LinkPopover.svelte';
import { DOC_REF_PREFIX } from './doc-ref.svelte.js';

/** @typedef {import('@milkdown/kit/prose/view').EditorView} EditorView */
/** @typedef {import('@milkdown/kit/prose/state').EditorState} EditorState */

/**
 * Opens a link's target: a writing for doc references, the browser otherwise.
 * @param {string} href
 */
export function openHref(href) {
  if (!href) return;
  if (href.startsWith(DOC_REF_PREFIX)) {
    const name = decodeURIComponent(href.slice(DOC_REF_PREFIX.length));
    goto(resolve(`/${encodeURIComponent(name)}`));
    return;
  }
  openUrl(href);
}

/**
 * Turns what the writer typed into an href: bare domains get https://,
 * bare email addresses get mailto:, anything with a scheme or path is kept.
 * @param {string} raw
 */
function normalizeHref(raw) {
  const href = raw.trim();
  if (!href || /^[a-z][a-z\d+.-]*:/i.test(href) || /^[#/.]/.test(href)) return href;
  if (/^[^\s@/]+@[^\s@/]+\.[^\s@/]+$/.test(href)) return `mailto:${href}`;
  if (/^[^\s/]+\.[a-z]{2,}(?:[/?#:]|$)/i.test(href)) return `https://${href}`;
  return href;
}

/**
 * The link mark at `pos` and the full range it covers, if the caret touches one.
 * @param {EditorState} state
 * @param {number} pos
 * @returns {{ from: number, to: number, mark: import('@milkdown/kit/prose/model').Mark } | null}
 */
function linkAt(state, pos) {
  const linkType = state.schema.marks.link;
  const at = state.doc.resolve(pos);
  if (!linkType || !at.parent.inlineContent) return null;
  const mark =
    linkType.isInSet(at.marks()) ??
    linkType.isInSet(at.nodeBefore?.marks ?? []) ??
    linkType.isInSet(at.nodeAfter?.marks ?? []);
  if (!mark) return null;

  // Walk the text block for the run of siblings carrying this exact mark
  // that contains the caret; formatting inside a link splits it into several nodes.
  const offset = at.parentOffset;
  /** @type {number | null} */
  let runStart = null;
  let runEnd = 0;
  /** @type {{ from: number, to: number } | null} */
  let found = null;
  at.parent.forEach((child, childOffset) => {
    if (found) return;
    if (mark.isInSet(child.marks)) {
      if (runStart === null) runStart = childOffset;
      runEnd = childOffset + child.nodeSize;
      return;
    }
    if (runStart !== null && runStart <= offset && offset <= runEnd) found = { from: runStart, to: runEnd };
    runStart = null;
  });
  if (!found && runStart !== null && runStart <= offset && offset <= runEnd) {
    found = { from: runStart, to: runEnd };
  }
  if (!found) return null;
  const start = at.start();
  return { from: start + found.from, to: start + found.to, mark };
}

/**
 * Asks the open editor to edit the link at the caret or selection, or to
 * insert a new one. Set while a link popover is mounted.
 * @type {((view: EditorView) => boolean) | null}
 */
let startEditing = null;

/**
 * Starts editing a link in `view` (⌘K, the slash menu).
 * @param {EditorView} view
 * @param {{ allowInsert?: boolean }} [options] - Without it, an empty caret outside a link does nothing.
 * @returns {boolean} Whether the editor took it.
 */
export function editLink(view, { allowInsert = true } = {}) {
  const { selection } = view.state;
  if (!allowInsert && selection.empty && !linkAt(view.state, selection.from)) return false;
  return startEditing?.(view) ?? false;
}

/**
 * Typora-style link editing. With the caret in a link, a small toolbar under
 * it shows where the link goes, with open, edit and remove. ⌘K (or the slash
 * menu) edits the address, links the selection, or inserts a new link.
 */
export const linkPopover = prose(() => {
  const content = document.createElement('div');
  content.classList.add('link-popover-portal');
  content.style.position = 'absolute';
  content.style.zIndex = '50';

  /** @type {{ mode: 'hidden' | 'view' | 'edit', href: string, needsText: boolean }} */
  const ui = $state({ mode: 'hidden', href: '', needsText: false });

  /** The document range the popover acts on: a link, a selection, or the caret for a new link. */
  let range = { from: 0, to: 0 };
  /** @type {EditorView | null} */
  let editorView = null;

  const provider = new TooltipProvider({
    content,
    offset: 6,
    floatingUIOptions: { placement: 'bottom-start' },
  });

  /** @param {EditorView} view */
  function show(view) {
    const anchor = {
      getBoundingClientRect: () => {
        const size = view.state.doc.content.size;
        return posToDOMRect(view, Math.min(range.from, size), Math.min(range.to, size));
      },
    };
    provider.show(anchor, view);
  }

  function hide() {
    ui.mode = 'hidden';
    provider.hide();
  }

  /** Shows the toolbar when the caret sits in a link, hides it otherwise. @param {EditorView} view */
  function sync(view) {
    const { selection } = view.state;
    const link = selection.empty && view.hasFocus() ? linkAt(view.state, selection.from) : null;
    if (!link) return hide();
    const sameLink =
      ui.mode === 'view' && range.from === link.from && range.to === link.to && ui.href === link.mark.attrs.href;
    range = { from: link.from, to: link.to };
    ui.href = link.mark.attrs.href ?? '';
    ui.needsText = false;
    ui.mode = 'view';
    if (!sameLink) show(view);
  }

  /** @param {EditorView} view */
  function beginEdit(view) {
    const { state } = view;
    const { selection } = state;
    if (!(selection instanceof TextSelection) || !selection.$from.parent.inlineContent) return false;

    const link = linkAt(state, selection.from);
    if (!selection.empty) {
      range = { from: selection.from, to: selection.to };
      /** @type {string} */
      let href = '';
      state.doc.nodesBetween(range.from, range.to, (node) => {
        const mark = !href && state.schema.marks.link.isInSet(node.marks);
        if (mark) href = mark.attrs.href ?? '';
      });
      ui.href = href;
      ui.needsText = false;
    } else if (link) {
      range = { from: link.from, to: link.to };
      ui.href = link.mark.attrs.href ?? '';
      ui.needsText = false;
    } else {
      range = { from: selection.from, to: selection.from };
      ui.href = '';
      ui.needsText = true;
    }
    editorView = view;
    ui.mode = 'edit';
    show(view);
    return true;
  }

  /** Back to the editor, caret at the end of the range. */
  function refocus() {
    const view = editorView;
    if (!view) return;
    const pos = Math.min(range.to, view.state.doc.content.size);
    view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, pos)));
    view.focus();
  }

  /**
   * @param {string} rawHref
   * @param {string} rawText
   */
  function apply(rawHref, rawText) {
    const view = editorView;
    if (!view) return;
    const href = normalizeHref(rawHref);
    const linkType = view.state.schema.marks.link;
    const tr = view.state.tr;

    if (ui.needsText) {
      const text = rawText.trim() || href;
      if (text) {
        const marks = href ? [linkType.create({ href })] : [];
        tr.insert(range.from, view.state.schema.text(text, marks));
        range = { from: range.from, to: range.from + text.length };
      }
    } else {
      tr.removeMark(range.from, range.to, linkType);
      if (href) tr.addMark(range.from, range.to, linkType.create({ href }));
    }
    // Typing right after the link shouldn't extend it.
    tr.removeStoredMark(linkType);
    view.dispatch(tr);
    ui.mode = 'hidden';
    provider.hide();
    refocus();
  }

  function cancel() {
    if (ui.mode !== 'edit') return;
    ui.mode = 'hidden';
    provider.hide();
    refocus();
  }

  function dismiss() {
    if (ui.mode !== 'edit') return;
    hide();
    // Focus may have gone back to the editor (a click in the text); show the
    // toolbar again if that click landed in a link.
    if (editorView?.hasFocus()) sync(editorView);
  }

  function remove() {
    const view = editorView;
    if (!view) return;
    view.dispatch(view.state.tr.removeMark(range.from, range.to, view.state.schema.marks.link));
    hide();
    view.focus();
  }

  const component = mount(LinkPopover, {
    target: content,
    props: {
      get mode() {
        return ui.mode;
      },
      get href() {
        return ui.href;
      },
      get label() {
        if (!ui.href.startsWith(DOC_REF_PREFIX)) return ui.href;
        return baseOf(decodeURIComponent(ui.href.slice(DOC_REF_PREFIX.length))).replace(/\.md$/i, '');
      },
      get isDocRef() {
        return ui.href.startsWith(DOC_REF_PREFIX);
      },
      get needsText() {
        return ui.needsText;
      },
      onOpen: () => openHref(ui.href),
      onEdit: () => editorView && beginEdit(editorView),
      onRemove: remove,
      onApply: apply,
      onCancel: cancel,
      onDismiss: dismiss,
    },
  });

  return new Plugin({
    view(view) {
      editorView = view;
      startEditing = beginEdit;
      (view.dom.parentElement ?? document.body).appendChild(content);
      return {
        update(view, prev) {
          editorView = view;
          if (ui.mode === 'edit') return;
          if (prev && prev.doc === view.state.doc && prev.selection === view.state.selection) return;
          sync(view);
        },
        destroy() {
          if (startEditing === beginEdit) startEditing = null;
          unmount(component);
          provider.destroy();
          content.remove();
        },
      };
    },
    props: {
      handleKeyDown(view, event) {
        if (event.key.toLowerCase() !== 'k' || !isMod(event) || event.shiftKey || event.altKey) return false;
        // With nothing to link, leave ⌘K to the command palette.
        return editLink(view, { allowInsert: false });
      },
      handleDOMEvents: {
        focus(view) {
          if (ui.mode !== 'edit') sync(view);
          return false;
        },
        blur(_view, event) {
          const next = /** @type {Node | null} */ (event.relatedTarget);
          if (ui.mode === 'view' && !(next && content.contains(next))) hide();
          return false;
        },
      },
    },
  });
});
