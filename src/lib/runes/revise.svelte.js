/**
 * @file Revise mode: the page compared with an earlier version of the same
 * writing. The editor plugin (plugins/revise.js) does the comparing and
 * keeps this up to date; the bar above the page reads it.
 */
class ReviseState {
  /** Whether the page is being compared with an earlier version. */
  open = $state(false);
  /** What it is compared with, in words ("the start of today"). */
  label = $state('');
  /** How many changes there are now. */
  count = $state(0);
  /** Which change holds the caret, 1-based; 0 when none does. */
  position = $state(0);

  reset() {
    this.open = false;
    this.label = '';
    this.count = 0;
    this.position = 0;
  }
}

export const reviseState = new ReviseState();
