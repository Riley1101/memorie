<script>
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { fileManager, baseOf, dirOf } from '$lib/runes/fs.svelte.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { toast } from '$lib/toast.js';
  import { deriveTitle, sanitizeTitle, isUntitled, UNTITLED } from '$lib/new-writing.js';
  import Editor from './editor.svelte';
  import RustEditor from './rust-editor.svelte';
  import EditorAiBox from './editor-ai-box.svelte';
  import EditorSlashMenu from './editor-slash-menu.svelte';
  import EditorDocRefMenu from './editor-doc-ref-menu.svelte';
  import { findAll, findDecorations, firstFrom, replaceCommands } from '$lib/editor/find.js';
  import { flattenBlocks } from '$lib/editor/decorations.js';
  import { mentionDecorations } from '$lib/editor/mentions.js';
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
   * Writes any pending edits before something renames the file. The engine's
   * session is keyed by path: after a rename the surface reopens under the new
   * name and reads it from disk, so whatever hadn't been saved would be lost.
   */
  async function flushBeforeRename() {
    if (useRust && rustTimer) await flushRustSave();
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
      await flushBeforeRename();
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
      await flushBeforeRename();
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

  /**
   * The selection the Rust surface last reported, which is what decides whether
   * the AI box is on screen and where. Milkdown mounts its own copy from a
   * ProseMirror widget, so this is only for the engine path.
   * @type {import('$lib/editor/dom.js').SelectionInfo | null}
   */
  let rustSelection = $state(null);

  /**
   * Find state for the engine path. The matches are not mapped through edits —
   * they are rescanned, which is what `$lib/editor/find.js` explains — so the
   * only state worth keeping is the search itself and which match is current.
   * @type {{ regex: RegExp, useGroups: boolean } | null}
   */
  let rustSearch = $state(null);
  /** @type {import('$lib/editor/find.js').FindMatch[]} */
  let rustMatches = $state([]);
  let rustCurrent = $state(-1);
  /**
   * Bumped whenever the engine path redraws, so the mentions are recomputed
   * once per edit rather than on every read of the derived value.
   */
  let rustRevision = $state(0);

  /**
   * Codex names underlined where they appear. Recomputed when the document or
   * the entries change, rather than mapped through the edit — the same choice
   * the find highlights make, and for the same reason.
   */
  let rustMentions = $derived.by(() => {
    void rustRevision;
    if (!useRust || codexEntities.length === 0) return [];
    return mentionDecorations(rustSurface?.snapshot()?.blocks ?? [], codexEntities);
  });

  /** What the surface draws: the mentions, with any search matches over them. */
  let rustDecorations = $derived(
    rustSearch ? [...rustMentions, ...findDecorations(rustMatches, rustCurrent)] : rustMentions
  );

  /**
   * Rescans for the active search, keeping the writer on the match they were
   * on — or the next one after it, when the one they were on has been edited
   * out from under them.
   * @param {{ block: number, offset: number }} [from]
   */
  function rustRescan(from) {
    const blocks = rustSurface?.snapshot()?.blocks ?? [];
    if (!rustSearch) {
      rustMatches = [];
      rustCurrent = -1;
      return { total: 0, current: -1 };
    }
    const anchor =
      from ??
      (rustMatches[rustCurrent]
        ? { block: rustMatches[rustCurrent].block, offset: rustMatches[rustCurrent].start }
        : (rustSurface?.snapshot()?.selection.head ?? { block: -1, offset: 0 }));
    rustMatches = findAll(blocks, rustSearch.regex);
    rustCurrent = firstFrom(
      rustMatches,
      flattenBlocks(blocks).map((block) => block.id),
      anchor
    );
    return { total: rustMatches.length, current: rustCurrent };
  }

  /** Puts the caret on the current match and scrolls to it. */
  function revealRustMatch() {
    const match = rustMatches[rustCurrent];
    if (match) rustSurface?.selectRange(match.block, match.start, match.end);
  }
  /** @type {ReturnType<typeof setTimeout> | null} */
  let rustTimer = null;

  /**
   * The URL for a link, for ⌘K and for the slash menu's link item.
   *
   * The engine surface draws no dialogs, so it asks for this. A prompt is the
   * placeholder: the link popover is still a Milkdown plugin, and porting it is
   * what replaces this.
   * @returns {string | null}
   */
  function askForLink() {
    const url = window.prompt('Link to');
    return url?.trim() ? url.trim() : null;
  }

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
    // Mentions are drawn from the blocks, so the first draw needs this too.
    rustRevision += 1;
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
    // A rescan rather than mapping anything through the edit: cheap enough over
    // the blocks the view already holds, and it cannot drift from them.
    rustRevision += 1;
    if (rustSearch) rustRescan();
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
    // The selection if there is one, the whole document if not — what the AI
    // edit actions act on, the same contract Milkdown registers.
    editorState.selectedText = () => {
      const selected = rustSurface?.selectionInfo()?.text;
      if (selected) return selected;
      const blocks = rustSurface?.snapshot()?.blocks ?? [];
      return plainTextOf(blocks);
    };
    editorState.focusEditor = () => rustSurface?.focus();
    editorState.find = {
      search: (regex, useGroups) => {
        rustSearch = regex ? { regex, useGroups } : null;
        const counts = rustRescan(rustSurface?.snapshot()?.selection.head);
        if (regex) revealRustMatch();
        return counts;
      },
      step: (direction) => {
        if (rustMatches.length === 0) return { total: 0, current: -1 };
        rustCurrent = (rustCurrent + direction + rustMatches.length) % rustMatches.length;
        revealRustMatch();
        return { total: rustMatches.length, current: rustCurrent };
      },
      replaceCurrent: async (replacement) => {
        const match = rustMatches[rustCurrent];
        if (!match) return { total: rustMatches.length, current: rustCurrent };
        await rustSurface?.runAsOneEdit(
          replaceCommands([match], replacement, rustSearch?.useGroups ?? false)
        );
        // Carry on from just past the replacement, not from its start, or the
        // next step lands back on text that was just put in.
        const counts = rustRescan({
          block: match.block,
          offset: match.start + replacement.length,
        });
        revealRustMatch();
        return counts;
      },
      replaceAll: async (replacement) => {
        const count = rustMatches.length;
        if (!count) return 0;
        // One edit, so one press of undo brings them all back.
        await rustSurface?.runAsOneEdit(
          replaceCommands(rustMatches, replacement, rustSearch?.useGroups ?? false)
        );
        rustRescan();
        return count;
      },
      selectedQuery: () => {
        const selected = rustSurface?.selectionInfo()?.text ?? '';
        return selected.includes('\n') ? '' : selected;
      },
    };
    return () => {
      if (editorState.flushSave === flushRustSave) editorState.flushSave = null;
      editorState.revealHeading = null;
      editorState.headingAbove = null;
      editorState.selectedText = null;
      editorState.focusEditor = null;
      editorState.find = null;
      rustSearch = null;
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
          decorations={rustDecorations}
          onselection={(info) => (rustSelection = info)}
          onlink={askForLink}
          onerror={(message) => toast.error('Editor error', message)}
        />
        <EditorAiBox info={rustSelection} surface={rustSurface} />
        <EditorSlashMenu info={rustSelection} surface={rustSurface} onlink={askForLink} />
        <EditorDocRefMenu info={rustSelection} surface={rustSurface} />
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
