import { editorViewCtx } from '@milkdown/kit/core';
import { editorState } from '$lib/runes/editor.svelte.js';

/**
 * @file The open document read as plain text, with a way back to editor
 * positions. Margin notes use it to find a note's words again; Revise mode
 * uses it to compare the page with an earlier version.
 */

/** @typedef {import('@milkdown/kit/prose/model').Node} PMNode */
/** @typedef {import('@milkdown/kit/prose/view').EditorView} EditorView */

/** Placeholder for inline nodes that aren't text (images, hard breaks). */
export const LEAF = '￼';

/**
 * The text of a stretch of the document, written the same way `indexText`
 * writes it: blocks joined by "\n", other inline nodes as `LEAF`.
 * @param {PMNode} doc @param {number} from @param {number} to
 */
export function textOf(doc, from, to) {
  return doc.textBetween(from, to, '\n', LEAF);
}

/**
 * The document's text with a map back to positions: one entry per inline
 * node, blocks joined by "\n".
 * @param {PMNode} doc
 */
export function indexText(doc) {
  let text = '';
  /** @type {{ start: number, pos: number, length: number }[]} */
  const segments = [];
  /** Each textblock: where its text starts, and where its content starts. */
  /** @type {{ start: number, pos: number }[]} */
  const blocks = [];
  let first = true;
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    if (!first) text += '\n';
    first = false;
    blocks.push({ start: text.length, pos: pos + 1 });
    node.forEach((child, offset) => {
      const piece = child.isText ? (child.text ?? '') : LEAF;
      segments.push({ start: text.length, pos: pos + 1 + offset, length: piece.length });
      text += piece;
    });
    return false;
  });

  /** Document position of the character at `offset`, or null between blocks. @param {number} offset */
  function posAt(offset) {
    let lo = 0;
    let hi = segments.length - 1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      const seg = segments[mid];
      if (offset < seg.start) hi = mid - 1;
      else if (offset >= seg.start + seg.length) lo = mid + 1;
      else return seg.pos + (offset - seg.start);
    }
    return null;
  }

  /**
   * Document position just before the character at `offset`. Unlike `posAt`
   * it always answers: on a block break it gives the end of the block before,
   * and in an empty block, that block's inside.
   * @param {number} offset
   */
  function boundaryAt(offset) {
    let lo = 0;
    let hi = blocks.length - 1;
    let at = 0;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (blocks[mid].start <= offset) {
        at = mid;
        lo = mid + 1;
      } else hi = mid - 1;
    }
    const block = blocks[at];
    if (!block) return 0;
    const next = blocks[at + 1];
    const length = (next ? next.start - 1 : text.length) - block.start;
    return block.pos + Math.min(offset - block.start, length);
  }

  return { text, posAt, boundaryAt };
}

/** The open editor's view, if a writing is open. @returns {EditorView | null} */
export function openView() {
  /** @type {EditorView | null} */
  let view = null;
  editorState.editor?.action((ctx) => {
    view = ctx.get(editorViewCtx);
  });
  return view;
}
