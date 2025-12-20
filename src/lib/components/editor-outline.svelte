<script>
  /**
   * EditorOutline component - generates a table of contents from markdown headings
   * @param {string} body - The markdown content to parse
   */
  let { body = '' } = $props();

  /**
   * @typedef {Object} Heading
   * @property {number} level - Heading level (1-6)
   * @property {string} text - Heading text content
   * @property {string} id - Generated ID for linking
   */

  /**
   * Extract headings from markdown content
   * @type {Heading[]}
   */
  let headings = $derived.by(() => {
    const lines = body.split('\n');
    const result = [];

    for (const line of lines) {
      const match = line.match(/^(#{1,6})\s+(.+)$/);
      if (match) {
        const level = match[1].length;
        const text = match[2].trim();
        const id = text
          .toLowerCase()
          .replace(/[^\w\s-]/g, '')
          .replace(/\s+/g, '-');

        result.push({ level, text, id });
      }
    }

    return result;
  });

  /**
   * Scroll to heading in the editor
   * @param {string} id - The heading ID to scroll to
   */
  function scrollToHeading(id) {
    const element = document.getElementById(id);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }
</script>

<div class="p-4 space-y-1">
  {#if headings.length > 0}
    <nav class="space-y-1">
      {#each headings as heading (heading.id)}
        <button
          onclick={() => scrollToHeading(heading.id)}
          class="block w-full text-left text-sm hover:text-foreground transition-colors text-muted-foreground"
          style="padding-left: {(heading.level - 1) * 0.75}rem"
        >
          {heading.text}
        </button>
      {/each}
    </nav>
  {:else}
    <p class="text-sm text-muted-foreground">No headings found</p>
  {/if}
</div>
