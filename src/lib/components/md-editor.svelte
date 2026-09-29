<script>
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager, baseOf, dirOf } from '$lib/runes/fs.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { toast } from '$lib/toast.js';
  import { deriveTitle, sanitizeTitle, isUntitled, UNTITLED } from '$lib/new-writing.js';
  import Editor from './editor.svelte';
  import RustEditor from './rust-editor.svelte';
  import { save as rustSave } from '$lib/rust-editor.js';
  import { invalidateAll, goto, beforeNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { resolve } from '$app/paths';
  import { editorViewCtx } from '@milkdown/kit/core';
  import { getMarkdown } from '@milkdown/kit/utils';
  import { untrack } from 'svelte';
  import { parseWriting, serializeWriting } from '$lib/front-matter.js';
  import { formatDate, formatTime } from '$lib/utils.js';
  import { writingState } from '$lib/runes/writing.svelte.js';
  import { codexManager } from '$lib/runes/codex.svelte.js';

  /** Strip .md for display */
  function stripMd(name) {
    if (!name || typeof name !== 'string') return '';
    return name.replace(/\.md$/i, '').trim();
  }

  /**
   * @type {{
   *   fileName: string,
   *   body?: string,
   *   isDraft?: boolean,
   *   docKey?: number,
   * }}
   * `fileName` is the real path, or for a draft the planned path (nothing on
   * disk yet). `docKey` changes only on real navigation between documents, so
   * a rename can move the URL without remounting the editor.
   */
  let { fileName, body: content, isDraft = false, docKey = 0 } = $props();

  // The editor only ever sees the body; metadata lives in editorState and is
  // put back on save.
  let parsed = $derived(parseWriting(content));

  /** @param {string} markdown - Body from the editor. */
  function fileContent(markdown) {
    return serializeWriting(
      editorState.meta,
      editorState.metaExtra,
      markdown,
      editorState.metaOriginal
    );
  }

  // Reload metadata when the document changes or is restored to another version.
  $effect(() => {
    void docKey;
    void appState.ui.editorVersion;
    untrack(() => editorState.loadMeta(parsed));
  });

  /** The path we are currently saving to. Follows the prop, and any rename we perform. */
  let currentName = $state(fileName);
  let draft = $state(isDraft);

  /** Display-only title (no .md, no folders); user edits this */
  let displayTitle = $state('');
  /** Once the writer edits the title, we stop deriving it from the body. */
  let titleTouched = $state(false);
  /** Drafts keep following the first line until the title is touched. */
  let autoTitle = $state(isDraft);
  let isRenaming = $state(false);
  let titleInput = $state(/** @type {HTMLInputElement | null} */ (null));

  /** Byline under the title (Paper style only): folder · last modified. */
  let byline = $derived.by(() => {
    const dir = dirOf(currentName);
    const where = dir ? dir.replaceAll('/', ' / ') : '';
    const modified = fileManager.files.find((f) => f.name === currentName)?.last_modified;
    const stamp = modified ?? Math.floor(Date.now() / 1000);
    const when = `${formatDate(stamp)}, ${formatTime(stamp)}`;
    return where ? `${where} · ${when}` : when;
  });

  $effect(() => {
    currentName = fileName;
    draft = isDraft;
  });

  // Reset per-document state on real navigation only (not on our own renames).
  $effect(() => {
    void docKey;
    untrack(() => {
      titleTouched = false;
      autoTitle = isDraft;
      displayTitle = isDraft ? '' : stripMd(baseOf(fileName));
    });
  });

  $effect(() => {
    editorState.setName(displayTitle || UNTITLED);
  });

  /** The binder the writing is in; '' at the top level. */
  let binder = $derived(currentName.includes('/') ? currentName.split('/')[0] : '');

  $effect(() => {
    if (writingState.codexHighlight) codexManager.load(binder || null);
  });

  let codexEntities = $derived(
    writingState.codexHighlight ? codexManager.entities(binder || null) : []
  );

  function focusEditor() {
    editorState.editor?.action((ctx) => {
      const view = ctx.get(editorViewCtx);
      view.focus();
    });
  }

  /**
   * Moves the URL to `newName` without remounting the editor. Skipped if the
   * writer already navigated elsewhere (e.g. a flush on the way out), which
   * `docKey` detects even when the next page has the same URL (draft → draft).
   * @param {string} newName
   * @param {number} startKey - `docKey` when the save began.
   */
  async function followUrl(newName, startKey) {
    const here = page.params.lexical ?? '';
    const stillHere = docKey === startKey && (here === fileName || here === currentName);
    if (!stillHere) return;
    currentName = newName;
    displayTitle = stripMd(baseOf(newName));

    editorState.silentNavigation = true;
    try {
      await goto(resolve(`/${encodeURIComponent(newName)}`), {
        replaceState: true,
        keepFocus: true,
        noScroll: true,
      });
    } finally {
      editorState.silentNavigation = false;
    }
  }

  /** In-flight first save of a draft; later saves wait on it instead of creating twice. */
  let materializing = /** @type {Promise<void> | null} */ (null);

  /**
   * First save of a draft: pick a name and create the file. Safe to call
   * repeatedly; only the first call creates anything.
   * @param {string} markdown
   * @returns {Promise<void>}
   */
  function materialize(markdown) {
    if (materializing) return materializing;
    const startKey = docKey;
    const dir = dirOf(currentName);
    const typed = titleTouched ? sanitizeTitle(displayTitle) : '';
    const wanted = typed || deriveTitle(markdown) || UNTITLED;
    const base = fileManager.uniqueName(dir, wanted);
    const full = dir ? `${dir}/${base}.md` : `${base}.md`;

    materializing = (async () => {
      try {
        await fileManager.createFileAt(full, fileContent(markdown));
        if (docKey !== startKey) return; // writer already moved on
        draft = false;
        autoTitle = !typed;
        await followUrl(full, startKey);
      } finally {
        materializing = null;
      }
    })();
    return materializing;
  }

  /**
   * Keeps an auto-titled file's name in step with its first line.
   * @param {string} markdown
   */
  async function maybeAutoRename(markdown) {
    if (!autoTitle || isRenaming) return;
    const startKey = docKey;
    const derived = deriveTitle(markdown);
    const currentBase = stripMd(baseOf(currentName));
    if (!derived) return;
    if (derived === currentBase) return;
    // "Untitled 3" -> "Untitled" is not an improvement worth a rename.
    if (isUntitled(derived) && isUntitled(currentBase)) return;

    const base = fileManager.uniqueName(dirOf(currentName), derived);
    if (base === currentBase) return;

    isRenaming = true;
    try {
      const renamed = await fileManager.renameFile(currentName, base);
      if (renamed) await followUrl(renamed, startKey);
    } finally {
      isRenaming = false;
    }
  }

  /**
   * Renames the file when the title loses focus or Enter is pressed.
   */
  async function commitTitle() {
    if (isRenaming) return;
    const cleaned = sanitizeTitle(displayTitle);
    const currentBase = stripMd(baseOf(currentName));

    if (draft) {
      // Nothing on disk yet. A typed title is enough to create the file so it
      // isn't lost if the writer leaves before typing a body.
      if (cleaned) {
        titleTouched = true;
        displayTitle = cleaned;
        try {
          const markdown = editorState.editor?.action(getMarkdown()) ?? '';
          await materialize(markdown);
        } catch (e) {
          toast.error('Could not create writing', e);
        }
      }
      return;
    }

    if (!cleaned || cleaned === currentBase) {
      displayTitle = currentBase;
      return;
    }

    titleTouched = true;
    autoTitle = false;
    isRenaming = true;
    const startKey = docKey;
    try {
      const renamed = await fileManager.renameFile(currentName, cleaned);
      if (renamed) {
        await followUrl(renamed, startKey);
      } else {
        displayTitle = currentBase;
      }
    } finally {
      isRenaming = false;
    }
  }

  /** @param {KeyboardEvent} e */
  function handleTitleKeydown(e) {
    if (e.key === 'Enter' || e.key === 'ArrowDown') {
      e.preventDefault();
      titleInput?.blur();
      focusEditor();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      displayTitle = draft ? '' : stripMd(baseOf(currentName));
      titleInput?.blur();
    }
  }

  function handleTitleInput() {
    titleTouched = true;
    autoTitle = false;
  }

  /**
   * The Rust engine as the editing surface, instead of Milkdown. A draft has no
   * file yet and the engine has nothing to open, so drafts stay on Milkdown
   * until their first save has given them a name.
   */
  let useRust = $derived(writingState.rustEditor && !draft);

  /** @type {RustEditor | null} */
  let rustSurface = $state(null);
  /** @type {ReturnType<typeof setTimeout> | null} */
  let rustTimer = null;

  /**
   * The document as text, for the word count and the progress log.
   * @param {import('$lib/rust-editor.js').EditorBlock[]} blocks
   * @returns {string}
   */
  function plainTextOf(blocks) {
    return blocks
      .map((block) => {
        const own = (block.runs ?? []).map((run) => run.text ?? '').join('') || (block.text ?? '');
        const nested = plainTextOf([
          ...(block.children ?? []),
          ...(block.items ?? []).flatMap((item) => item.blocks),
          ...(block.rows ?? []).flatMap((row) => row.cells.flat()),
        ]);
        return [own, nested].filter(Boolean).join('\n\n');
      })
      .filter(Boolean)
      .join('\n\n');
  }

  /** @param {import('$lib/rust-editor.js').EditorStateView} state */
  function onRustState(state) {
    // The engine already knows the headings. `pos` carries the block id here,
    // which is what the outline hands back when one is clicked.
    editorState.setHeadings(
      state.outline.map((heading) => ({
        level: heading.level,
        text: heading.text,
        pos: heading.id,
      }))
    );
    writingState.resetBaseline();
    writingState.updateDocument(plainTextOf(state.blocks));
  }

  /** @param {import('$lib/rust-editor.js').EditorUpdate} update */
  function onRustUpdate(update) {
    if (update.outline) {
      editorState.setHeadings(
        update.outline.map((heading) => ({
          level: heading.level,
          text: heading.text,
          pos: heading.id,
        }))
      );
    }
  }

  /** Writes through `editor_save`, which takes the same path `update_file` does. */
  async function flushRustSave() {
    if (rustTimer) {
      clearTimeout(rustTimer);
      rustTimer = null;
    }
    editorState.setSaveStatus({ status: 'saving' });
    try {
      await rustSave(currentName);
      await fileManager.getRecents();
      editorState.setSaveStatus({ lastSaved: new Date(), status: 'saved' });
      const state = rustSurface?.snapshot();
      if (state) writingState.updateDocument(plainTextOf(state.blocks));
    } catch (e) {
      editorState.setSaveStatus({ status: 'error' });
      toast.error('Save failed', e);
    }
  }

  /** Autosave, on the same debounce the Milkdown editor uses. */
  function scheduleRustSave() {
    editorState.setSaveStatus({ status: 'unsaved' });
    if (rustTimer) clearTimeout(rustTimer);
    rustTimer = setTimeout(flushRustSave, 2000);
  }

  // ⌘S, the command bar and the outline all go through whichever surface is
  // mounted, so they don't have to know which one that is.
  $effect(() => {
    if (!useRust) return;
    editorState.flushSave = flushRustSave;
    editorState.revealHeading = (heading) => rustSurface?.goToBlock(heading.pos);
    editorState.headingAbove = (heading, threshold) =>
      rustSurface?.blockAbove(heading.pos, threshold) ?? null;
    return () => {
      if (editorState.flushSave === flushRustSave) editorState.flushSave = null;
      editorState.revealHeading = null;
      editorState.headingAbove = null;
    };
  });

  // Leaving the page with a pending autosave would lose the last words typed.
  // This runs before anything unmounts, so the engine's session is still open.
  beforeNavigate(() => {
    if (useRust && rustTimer) flushRustSave();
  });

  /**
   * Save callback for the editor. Drafts are created on their first non-empty
   * save; existing files are written in place and, if still auto-titled,
   * renamed to match their first line.
   * @param {string} markdown
   */
  async function onSave(markdown) {
    if (!editorState.editor) return;
    try {
      if (materializing) await materializing;
      if (draft) {
        if (!markdown.trim()) return;
        await materialize(markdown);
        return;
      }
      await fileManager.saveFile(currentName, fileContent(markdown));
      await maybeAutoRename(markdown);
      invalidateAll();
    } catch (e) {
      toast.error('Save failed', e);
      throw e;
    }
  }
</script>

<div class="writing-area">
  <input
    bind:this={titleInput}
    type="text"
    class="writing-area__title w-full"
    placeholder={UNTITLED}
    aria-label="Writing title"
    bind:value={displayTitle}
    oninput={handleTitleInput}
    onblur={commitTitle}
    onkeydown={handleTitleKeydown}
    spellcheck="false"
    autocomplete="off"
  />
  <p class="writing-area__byline" aria-hidden="true">{byline}</p>
  <div class="writing-area__ornament" aria-hidden="true">❧</div>
  <div class="writing-area__body">
    {#key `${docKey}:${appState.ui.editorVersion}`}
      {#if useRust}
        <!-- The engine owns the document here: no Milkdown, no second model. -->
        <RustEditor
          bind:this={rustSurface}
          name={currentName}
          spellcheck={writingState.spellcheck}
          class="markdown w-full"
          onstate={onRustState}
          onupdate={onRustUpdate}
          onedit={scheduleRustSave}
          onerror={(message) => toast.error('Editor error', message)}
        />
      {:else}
        <Editor defaultValue={parsed.body} {onSave} autofocus={draft} {codexEntities} />
      {/if}
    {/key}
  </div>
</div>

<style>
  .writing-area {
    padding-bottom: 0;
  }

  /* Byline and ornament are Paper-only; see .style-paper in app.css. */
  .writing-area__byline,
  .writing-area__ornament {
    display: none;
  }

  .writing-area__body {
    margin-top: var(--writer-gap-title, 0.75rem);
  }

  :global(.ProseMirror:focus) {
    outline: none;
  }

  :global(.ProseMirror) {
    min-height: 280px;
  }
</style>
