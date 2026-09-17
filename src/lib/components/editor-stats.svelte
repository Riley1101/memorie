<script>
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import * as Popover from '$lib/components/ui/popover/index.js';
  import { appState } from '$lib/runes/app.svelte.js';
  import { fileManager } from '$lib/runes/fs.svelte.js';
  import {
    writingState,
    countWords,
    markdownToPlainText,
  } from '$lib/runes/writing.svelte.js';

  /** @type {{ fileName: string, isDraft?: boolean }} */
  let { fileName, isDraft = false } = $props();

  const WORDS_PER_MINUTE = 238;
  const format = new Intl.NumberFormat();

  let open = $state(false);

  /** Top-level folder of the open writing, or null for loose writings. */
  let binder = $derived(fileName.includes('/') ? fileName.slice(0, fileName.indexOf('/')) : null);

  /** Words in the binder's other writings; null while loading or not yet asked. */
  let otherBinderWords = $state(/** @type {number | null} */ (null));

  let binderWords = $derived(
    otherBinderWords === null ? null : otherBinderWords + writingState.docWords
  );
  let projectGoal = $derived(binder ? (writingState.projectGoals[binder] ?? 0) : 0);

  let readingMinutes = $derived(
    writingState.docWords === 0 ? 0 : Math.max(1, Math.round(writingState.docWords / WORDS_PER_MINUTE))
  );

  /** Reads the binder's other writings once per popover open. */
  async function loadBinderWords() {
    if (!binder) return;
    otherBinderWords = null;
    const names = fileManager
      .filesUnder(binder)
      .map((f) => f.name)
      .filter((name) => name !== fileName);
    try {
      /** @type {[string, { Ok?: string, Err?: string } | string][]} */
      const results = await invoke('read_files', { names });
      let total = 0;
      for (const [, result] of results) {
        const text = typeof result === 'string' ? result : result?.Ok;
        if (text) total += countWords(markdownToPlainText(text));
      }
      otherBinderWords = total;
    } catch (e) {
      console.error('Could not count binder words:', e);
      otherBinderWords = 0;
    }
  }

  // Only re-count when the popover opens, not whenever the file list refreshes.
  $effect(() => {
    if (open) untrack(loadBinderWords);
  });

  /** @param {number} value @param {number} goal */
  function percent(value, goal) {
    return goal ? Math.min(100, Math.round((value / goal) * 100)) : 0;
  }

  let label = $derived(
    writingState.selectionWords
      ? `${format.format(writingState.selectionWords)} of ${format.format(writingState.docWords)} words`
      : `${format.format(writingState.docWords)} ${writingState.docWords === 1 ? 'word' : 'words'}`
  );
</script>

{#snippet goalBar(/** @type {number} */ value, /** @type {number} */ goal)}
  <div
    class="h-1 w-full rounded-full bg-muted overflow-hidden"
    role="progressbar"
    aria-valuemin={0}
    aria-valuemax={goal}
    aria-valuenow={Math.min(value, goal)}
  >
    <div
      class="h-full rounded-full transition-[width] duration-300 {value >= goal
        ? 'bg-success'
        : 'bg-primary'}"
      style="width: {percent(value, goal)}%"
    ></div>
  </div>
{/snippet}

{#snippet goalInput(
  /** @type {string} */ id,
  /** @type {number} */ value,
  /** @type {(n: number) => void} */ onchange
)}
  <input
    {id}
    type="number"
    min="0"
    step="100"
    placeholder="No goal"
    value={value || ''}
    onchange={(e) => onchange(Number(e.currentTarget.value))}
    class="w-24 h-7 rounded-md border border-border bg-transparent px-2 text-right text-xs tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
  />
{/snippet}

<Popover.Root bind:open>
  <Popover.Trigger
    class="flex items-center gap-2 px-2 -mx-2 h-7 rounded-md tracking-wide tabular-nums hover:text-foreground hover:bg-foreground/5 transition-colors"
    aria-label="Word count and goals"
  >
    <span>{label}</span>
    {#if writingState.dailyGoal}
      <span
        class="text-muted-foreground/60"
        title="Today: {format.format(writingState.todayWords)} of {format.format(writingState.dailyGoal)}"
      >
        · {percent(writingState.todayWords, writingState.dailyGoal)}% today
      </span>
    {/if}
  </Popover.Trigger>
  <Popover.Content
    side="top"
    align="start"
    class="{appState.ui.theme} w-80 font-sans text-sm"
    portalProps={{}}
  >
    <dl class="grid grid-cols-3 gap-3 pb-4 border-b border-border/50">
      <div>
        <dt class="text-xs text-muted-foreground">Words</dt>
        <dd class="text-lg tabular-nums">{format.format(writingState.docWords)}</dd>
      </div>
      <div>
        <dt class="text-xs text-muted-foreground">Characters</dt>
        <dd class="text-lg tabular-nums">{format.format(writingState.docChars)}</dd>
      </div>
      <div>
        <dt class="text-xs text-muted-foreground">Reading</dt>
        <dd class="text-lg tabular-nums">{readingMinutes} min</dd>
      </div>
    </dl>

    <div class="pt-4 space-y-2">
      <div class="flex items-center justify-between gap-3">
        <label for="daily-goal" class="font-medium">Today</label>
        {@render goalInput('daily-goal', writingState.dailyGoal, (n) => writingState.setDailyGoal(n))}
      </div>
      <p class="text-xs text-muted-foreground tabular-nums">
        {format.format(writingState.todayWords)}
        {#if writingState.dailyGoal}
          of {format.format(writingState.dailyGoal)} words
        {:else}
          words written today
        {/if}
      </p>
      {#if writingState.dailyGoal}
        {@render goalBar(writingState.todayWords, writingState.dailyGoal)}
      {/if}
    </div>

    {#if binder && !isDraft}
      <div class="pt-4 mt-4 border-t border-border/50 space-y-2">
        <div class="flex items-center justify-between gap-3">
          <label for="project-goal" class="font-medium truncate" title={binder}>{binder}</label>
          {@render goalInput('project-goal', projectGoal, (n) => writingState.setProjectGoal(binder, n))}
        </div>
        <p class="text-xs text-muted-foreground tabular-nums">
          {#if binderWords === null}
            Counting…
          {:else if projectGoal}
            {format.format(binderWords)} of {format.format(projectGoal)} words
          {:else}
            {format.format(binderWords)} words in this binder
          {/if}
        </p>
        {#if projectGoal && binderWords !== null}
          {@render goalBar(binderWords, projectGoal)}
        {/if}
      </div>
    {/if}
  </Popover.Content>
</Popover.Root>
