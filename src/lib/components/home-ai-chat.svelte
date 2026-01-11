<script>
  import * as Item from '$lib/components/ui/item/index.js';
  import FileIcon from '@lucide/svelte/icons/file';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

  /**
   * Represents the structure of the "Thing" parent object.
   * (Update this definition based on your actual Rust struct for Thing)
   * @typedef {Object} Thing
   */

  /**
   * Represents a result returned from a search query.
   * Corresponds to the Rust struct `SearchResult`.
   *
   * @typedef {Object} SearchResult
   * @property {Thing} parent - The parent entity associated with this result.
   * @property {string} content - The main textual content found.
   * @property {number} sequence - The sequence index (usize maps to number in JS).
   * @property {string} title - The title of the result.
   */

  /** @type {{result: {
    data: SearchResult[]
  }}} */
  let { result } = $props();
</script>

<div class="flex flex-col gap-2">
  {#each result?.data as item}
    <Item.Root variant="outline" size="sm">
      {#snippet child({ props })}
        <a href={`/${item.title}`} {...props}>
          <Item.Media>
            <FileIcon class="size-4 text-muted-foreground" />
          </Item.Media>
          <Item.Content>
            <Item.Title class="line-clamp-1">
              {item?.title}
            </Item.Title>
            <Item.Description class="line-clamp-1">
              {item?.content}
            </Item.Description>
          </Item.Content>
          <Item.Actions>
            <ChevronRightIcon class="size-4" />
          </Item.Actions>
        </a>
      {/snippet}
    </Item.Root>
  {/each}
</div>
