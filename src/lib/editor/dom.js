/**
 * @file Mapping between a `contenteditable` DOM and engine positions.
 *
 * This is the piece that decides whether Memorie can drop Milkdown: everything
 * else in a view is rendering, but a caret has to survive the browser doing
 * what it likes to the DOM — composition, autocorrect, drag and drop.
 *
 * Two rules hold the mapping together:
 *
 * 1. **Every block element carries `data-block-id`.** A position is that id
 *    plus an offset, so a caret never depends on an element's position in the
 *    tree.
 * 2. **Offsets are UTF-16 code units**, matching `String.prototype.length` and
 *    the wire format. An atom — an image, a hard break — is one unit, and is
 *    marked in the DOM with `data-atom` so it can be counted without being
 *    read.
 */

/** @typedef {{ block: number, offset: number }} EnginePosition */
/** @typedef {{ anchor: EnginePosition, head: EnginePosition }} EngineSelection */

/** Nodes that carry length: text, and atoms standing in for one unit. */
const LENGTH_NODES = NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT;

/** @param {Node} node */
function isAtom(node) {
  return (
    node.nodeType === Node.ELEMENT_NODE && /** @type {Element} */ (node).hasAttribute('data-atom')
  );
}

/**
 * The block element a node sits in, or null when the node is outside the
 * editor. Returns the *innermost* block, so text in a quoted paragraph belongs
 * to that paragraph and not to the quote around it.
 * @param {Node | null} node
 * @param {HTMLElement} root
 * @returns {HTMLElement | null}
 */
export function blockElementOf(node, root) {
  const element =
    node?.nodeType === Node.TEXT_NODE ? node.parentElement : /** @type {Element | null} */ (node);
  const block = element?.closest('[data-block-id]');
  return block instanceof HTMLElement && root.contains(block) ? block : null;
}

/**
 * Walks the length-carrying nodes of one block, in document order, skipping
 * anything inside a nested block (a quote's paragraphs are their own blocks).
 * @param {HTMLElement} block
 */
function* lengthNodes(block) {
  const walker = document.createTreeWalker(block, LENGTH_NODES, {
    acceptNode(node) {
      if (node !== block && node.nodeType === Node.ELEMENT_NODE) {
        const element = /** @type {Element} */ (node);
        // A nested block's contents belong to that block, not to this one.
        if (element.hasAttribute('data-block-id')) return NodeFilter.FILTER_REJECT;
        if (element.hasAttribute('data-atom')) return NodeFilter.FILTER_ACCEPT;
        return NodeFilter.FILTER_SKIP;
      }
      return node.nodeType === Node.TEXT_NODE ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP;
    },
  });
  let node = walker.nextNode();
  while (node) {
    yield node;
    node = walker.nextNode();
  }
}

/**
 * The engine position of a DOM point.
 * @param {Node} node
 * @param {number} offset - A text offset, or a child index for an element.
 * @param {HTMLElement} root
 * @returns {EnginePosition | null}
 */
export function positionOf(node, offset, root) {
  const block = blockElementOf(node, root);
  if (!block) return null;
  const id = Number(block.dataset.blockId);

  // A point given as (element, childIndex) — which browsers do for an empty
  // block, or either side of an atom — is resolved to the node it precedes.
  let target = node;
  let targetOffset = offset;
  /**
   * Set when the point is past an element's last child. For the block itself
   * that is the end of the block; for an inline container — a mark's span,
   * which is where a browser puts the caret at a bold run's edge — it is the
   * end of what *that span* holds, and the runs after it must not be counted.
   */
  let pastEndOf = /** @type {Element | null} */ (null);
  if (node.nodeType === Node.ELEMENT_NODE) {
    const element = /** @type {Element} */ (node);
    if (offset >= element.childNodes.length) {
      if (element === block) return { block: id, offset: blockLength(block) };
      pastEndOf = element;
    } else {
      target = element.childNodes[offset];
      targetOffset = 0;
    }
  }

  let total = 0;
  /** The offset at the end of `pastEndOf`, once it is known. */
  let end = /** @type {number | null} */ (null);
  for (const current of lengthNodes(block)) {
    if (!pastEndOf && current === target) return { block: id, offset: total + targetOffset };
    // An empty container holds nothing to measure, so its end is the offset of
    // the first thing that follows it.
    if (
      pastEndOf &&
      end === null &&
      !pastEndOf.contains(current) &&
      pastEndOf.compareDocumentPosition(current) & Node.DOCUMENT_POSITION_FOLLOWING
    ) {
      end = total;
    }
    total += isAtom(current) ? 1 : (current.textContent ?? '').length;
    if (pastEndOf?.contains(current)) end = total;
  }
  // The point is in the block but after everything measurable (an empty block).
  return { block: id, offset: pastEndOf ? (end ?? total) : total };
}

/** The length of a block's content, in UTF-16 units. */
export function blockLength(block) {
  let total = 0;
  for (const node of lengthNodes(block)) {
    total += isAtom(node) ? 1 : (node.textContent ?? '').length;
  }
  return total;
}

/** A block's text as the DOM has it, which is what composition leaves behind. */
export function blockText(block) {
  let text = '';
  for (const node of lengthNodes(block)) {
    // An atom contributes one unit of *something*; ￼ is the Unicode
    // object-replacement character, which is exactly this job.
    text += isAtom(node) ? '￼' : (node.textContent ?? '');
  }
  return text;
}

/**
 * Reads the browser's selection as an engine selection, or null when it isn't
 * in the editor.
 * @param {HTMLElement} root
 * @returns {EngineSelection | null}
 */
export function readSelection(root) {
  const selection = document.getSelection();
  if (!selection || selection.rangeCount === 0 || !selection.anchorNode) return null;
  const anchor = positionOf(selection.anchorNode, selection.anchorOffset, root);
  const head = selection.focusNode
    ? positionOf(selection.focusNode, selection.focusOffset, root)
    : anchor;
  return anchor && head ? { anchor, head } : null;
}

/**
 * The DOM point for an offset inside a block.
 * @param {HTMLElement} block
 * @param {number} offset
 * @returns {{ node: Node, offset: number }}
 */
function domPointOf(block, offset) {
  let seen = 0;
  let last = /** @type {{ node: Node, offset: number }} */ ({ node: block, offset: 0 });
  for (const node of lengthNodes(block)) {
    const length = isAtom(node) ? 1 : (node.textContent ?? '').length;
    if (offset <= seen + length) {
      if (isAtom(node)) {
        // Before or after the atom, never inside it.
        const parent = node.parentNode ?? block;
        const index = Array.prototype.indexOf.call(parent.childNodes, node);
        return { node: parent, offset: offset <= seen ? index : index + 1 };
      }
      return { node, offset: offset - seen };
    }
    seen += length;
    last = isAtom(node) ? { node: node.parentNode ?? block, offset: 0 } : { node, offset: length };
  }
  return last;
}

/**
 * Puts the browser's selection where the engine says it is. Skipped silently
 * when the blocks aren't in the DOM (a render is still pending).
 * @param {HTMLElement} root
 * @param {EngineSelection} selection
 */
export function writeSelection(root, selection) {
  const anchorBlock = root.querySelector(`[data-block-id="${selection.anchor.block}"]`);
  const headBlock = root.querySelector(`[data-block-id="${selection.head.block}"]`);
  if (!(anchorBlock instanceof HTMLElement) || !(headBlock instanceof HTMLElement)) return;

  const anchor = domPointOf(anchorBlock, selection.anchor.offset);
  const head = domPointOf(headBlock, selection.head.offset);
  const current = document.getSelection();
  if (!current) return;
  // setBaseAndExtent keeps a backwards selection backwards, which
  // addRange + collapse would not.
  try {
    current.setBaseAndExtent(anchor.node, anchor.offset, head.node, head.offset);
  } catch {
    // A stale offset can be out of range for the node; a collapsed caret at
    // the block start is a better outcome than throwing inside an input event.
    current.collapse(anchorBlock, 0);
  }
}
