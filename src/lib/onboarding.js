/**
 * @file First-run welcome.
 *
 * The app's best features — the command bar, binders, the codex, the branching
 * history — are all invisible on a blank home screen, and until now were
 * described only in the README. A new writer gets one short writing that shows
 * them, in the place they are already looking.
 */

import { fileManager } from '$lib/runes/fs.svelte.js';
import { MOD_LABEL } from '$lib/keyboard.svelte.js';

const SEEN_KEY = 'welcomeCreated';
const WELCOME_NAME = 'Welcome to Memoire.md';

/** @returns {string} */
function welcomeBody() {
  const mod = MOD_LABEL;
  return `---
synopsis: "A short tour. Delete it whenever you like — it goes to the trash."
status: "To Do"
---

# Welcome to Memoire

This is a writing, and it works like every other one: it is plain Markdown on
your own disk, it saves itself as you type, and the title above follows your
first line until you name it yourself.

## Press : for anything

Type a colon anywhere in the page and the command bar opens. \`:export\` sends a
manuscript to Word, PDF or EPUB. \`:focus\` clears the room. \`:toc\` opens the
outline. \`:sync\` publishes. You never have to remember a shortcut — but
${mod}+/ lists them all.

## Binders hold long work

A binder is a folder on the home screen. Inside it, folders become chapters and
writings become scenes, nested as deep as the work needs. Drag things between
them; the corkboard view gives every scene a synopsis card and a status.

## The codex remembers your people

Add a character, a place or a faction to the codex and Memoire underlines them
wherever you write their name. \`:ai\` can then answer questions about your own
pages, on your own machine, if you turn the assistant on in Settings.

## Nothing is lost

Every save is a version, and the history is a tree, not a line — so an
abandoned direction is still there after you take another one. Deleted
writings wait in **Recently deleted** for thirty days.

Now delete this and start.
`;
}

/**
 * Creates the welcome writing the first time the app opens on an empty
 * library. Does nothing if the writer already has writings, or has seen it
 * before and deleted it.
 * @returns {Promise<string|null>} Path of the writing created, or null.
 */
export async function ensureWelcomeWriting() {
  if (typeof localStorage === 'undefined') return null;
  if (localStorage.getItem(SEEN_KEY) === 'true') return null;

  // Mark it seen first: a writer who deletes the welcome should never see it
  // come back, and a failure here is not worth retrying on every launch.
  localStorage.setItem(SEEN_KEY, 'true');

  // Only on a genuinely empty library, so an existing writer upgrading to this
  // version is never handed a file they did not ask for.
  if (fileManager.files.length > 0 || fileManager.binders.length > 0) return null;

  try {
    await fileManager.createFileAt(WELCOME_NAME, welcomeBody());
    return WELCOME_NAME;
  } catch (e) {
    console.error('Could not create the welcome writing:', e);
    return null;
  }
}
