/**
 * @file Which writing the spike surface should open.
 *
 * The editor page sets this before sending the writer to `/spike-editor`, so
 * the spike can be pointed at a real document without putting a throwaway
 * parameter in the app's URLs. Goes away with the spike.
 */

export const spikeTarget = $state({
  /** @type {string | null} */
  name: null,
});
