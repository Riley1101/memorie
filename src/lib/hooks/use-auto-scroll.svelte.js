/*
 * Installed from @ieedan/shadcn-svelte-extras
 */

/**
 * Use this on a vertically scrollable container to ensure that it automatically scrolls to the bottom of the content.
 *
 * ## Usage in Svelte
 * ```svelte
 * <script>
 *   import { UseAutoScroll } from '$lib/hooks/use-auto-scroll.js';
 *
 *   const autoScroll = new UseAutoScroll();
 * </script>
 *
 * <div>
 *   <div bind:this={autoScroll.ref}>
 *     {@render children?.()}
 *   </div>
 *   {#if !autoScroll.isAtBottom}
 *     <button on:click={() => autoScroll.scrollToBottom()}>
 *       Scroll To Bottom
 *     </button>
 *   {/if}
 * </div>
 * ```
 */
export class UseAutoScroll {
  constructor() {
    /** @type {HTMLElement | null} */
    this._ref = null;

    /** @type {number} */
    this._scrollY = 0;

    /** @type {boolean} */
    this._userHasScrolled = false;

    /** @type {number} */
    this.lastScrollHeight = 0;
  }

  /**
   * Sets the reference to the scrollable container.
   * @param {HTMLElement | undefined} ref
   */
  set ref(ref) {
    this._ref = ref || null;

    if (!this._ref) return;

    this.lastScrollHeight = this._ref.scrollHeight;

    this._ref.scrollTo(0, this._scrollY ? this._scrollY : this._ref.scrollHeight);

    this._ref.addEventListener('scroll', () => {
      if (!this._ref) return;

      this._scrollY = this._ref.scrollTop;
      this.disableAutoScroll();
    });

    window.addEventListener('resize', () => {
      this.scrollToBottom(true);
    });

    const observer = new MutationObserver(() => {
      if (!this._ref) return;

      if (this._ref.scrollHeight !== this.lastScrollHeight) {
        this.scrollToBottom(true);
      }

      this.lastScrollHeight = this._ref.scrollHeight;
    });

    observer.observe(this._ref, { childList: true, subtree: true });
  }

  /** @returns {HTMLElement | null} */
  get ref() {
    return this._ref;
  }

  /** @returns {number} */
  get scrollY() {
    return this._scrollY;
  }

  /**
   * Checks if the container is scrolled to the bottom.
   * @returns {boolean}
   */
  get isAtBottom() {
    if (!this._ref) return true;

    return this._scrollY + this._ref.offsetHeight >= this._ref.scrollHeight;
  }

  /**
   * Disables auto scrolling until the container is scrolled back to the bottom.
   */
  disableAutoScroll() {
    if (this.isAtBottom) {
      this._userHasScrolled = false;
    } else {
      this._userHasScrolled = true;
    }
  }

  /**
   * Scrolls the container to the bottom.
   * @param {boolean} [auto=false] - If true, will only scroll if user hasn't manually scrolled.
   */
  scrollToBottom(auto = false) {
    if (!this._ref) return;

    if (auto && this._userHasScrolled) return;

    this._ref.scrollTo(0, this._ref.scrollHeight);
  }
}
