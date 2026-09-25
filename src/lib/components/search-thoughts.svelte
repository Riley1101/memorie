<script>
  /**
   * What a notes search found, shown above the answer like a thought: one line
   * while collapsed, every passage the answer was built from when opened.
   */
  import * as Collapsible from "$lib/components/ui/collapsible/index.js";
  import SearchIcon from "@lucide/svelte/icons/search";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";

  /**
   * @type {{
   *   query: string,
   *   binder?: string | null,
   *   passages?: import('$lib/runes/llm.svelte.js').Passage[],
   *   elapsedMs?: number,
   *   onOpen: (title: string) => void,
   * }}
   */
  let { query, binder = null, passages, elapsedMs, onOpen } = $props();

  let open = $state(false);
  let searching = $derived(passages === undefined);
  let where = $derived(binder ?? "notes");

  /** `Novel/Chapter 3.md` -> `Chapter 3` */
  function noteLabel(title) {
    return (title ?? "").replace(/\.md$/, "").split("/").pop() || "Untitled";
  }

  /** @param {number} ms */
  function duration(ms) {
    return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`;
  }

  /**
   * The reranker's verdict in words. Its scores are logits: above 0 the passage
   * most likely answers the question, well below it probably doesn't.
   * @param {number | null} score
   */
  function verdict(score) {
    if (score === null || score === undefined) return null;
    if (score >= 3) return "answers it";
    if (score >= 0) return "likely relevant";
    if (score >= -5) return "loosely related";
    return "weak match";
  }
</script>

{#if searching}
  <div class="flex items-center gap-1.5 text-xs text-muted-foreground/70 font-sans">
    <SearchIcon strokeWidth={1.5} class="size-3 animate-pulse" />
    <span class="truncate">Searching {where} for “{query}”…</span>
  </div>
{:else if passages.length === 0}
  <div class="flex items-center gap-1.5 text-xs text-muted-foreground/70 font-sans">
    <SearchIcon strokeWidth={1.5} class="size-3" />
    <span class="truncate">Searched {where} for “{query}” · no matches</span>
  </div>
{:else}
  <Collapsible.Root bind:open class="font-sans">
    <Collapsible.Trigger
      class="group flex max-w-full items-center gap-1.5 text-xs text-muted-foreground/70 hover:text-foreground transition-colors"
    >
      <SearchIcon strokeWidth={1.5} class="size-3 shrink-0" />
      <span class="truncate">
        Searched {where} for “{query}” · {passages.length}
        {passages.length === 1 ? "passage" : "passages"}{elapsedMs !== undefined ? ` · ${duration(elapsedMs)}` : ""}
      </span>
      <ChevronRight
        strokeWidth={1.5}
        class="size-3 shrink-0 transition-transform {open ? 'rotate-90' : ''}"
      />
    </Collapsible.Trigger>

    <Collapsible.Content>
      <ol class="mt-2 ml-1.5 flex flex-col gap-2.5 border-l border-border/60 pl-3">
        {#each passages as passage, i (i)}
          {@const judged = verdict(passage.rerankScore)}
          <li>
            <button
              type="button"
              onclick={() => onOpen(passage.title)}
              title="Open {passage.title}"
              class="flex w-full flex-col gap-0.5 rounded-md text-left text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              <span class="flex min-w-0 items-baseline gap-1.5">
                <span class="shrink-0 tabular-nums text-muted-foreground/50">{i + 1}.</span>
                <span class="truncate font-medium text-foreground/80">
                  {noteLabel(passage.title)}{#if passage.section}<span class="font-normal text-muted-foreground/70"> › {passage.section}</span>{/if}
                </span>
                {#if passage.startLine > 0}
                  <span class="shrink-0 tabular-nums text-muted-foreground/50">
                    {passage.startLine === passage.endLine
                      ? `line ${passage.startLine}`
                      : `lines ${passage.startLine}–${passage.endLine}`}
                  </span>
                {/if}
              </span>
              <span class="line-clamp-2 pl-4 text-muted-foreground/80">{passage.excerpt}</span>
              <span class="pl-4 text-[0.6875rem] text-muted-foreground/50">
                by {passage.foundBy.join(" + ") || "search"}
                · {Math.round(passage.similarity * 100)}% similar{#if judged}<span title="Reranker score {passage.rerankScore.toFixed(1)}"> · {judged}</span>{/if}
              </span>
            </button>
          </li>
        {/each}
      </ol>
    </Collapsible.Content>
  </Collapsible.Root>
{/if}
