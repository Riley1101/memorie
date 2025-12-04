<script>
  /* eslint svelte/no-at-html-tags: "warn" */

  import { llmManager } from '@/runes/llm.svelte.js';
  import * as Chat from '$lib/components/ui/chat';
  import { marked } from 'marked';
  import { sanitizeMarkdown } from '$lib/utils';
  import SparkleIcon from '@lucide/svelte/icons/sparkles';
  import UserIcon from '@lucide/svelte/icons/user-round';
</script>

<Chat.List class="dark">
  {#each llmManager.messages as message, index (index)}
    <Chat.Bubble variant={message.role === 'user' ? 'sent' : 'received'}>
      <Chat.BubbleAvatar class="items-center justify-center text-muted-foreground">
        {#if message.role === 'user'}
          <UserIcon class="size-4" />
        {:else}
          <SparkleIcon class="size-4" />
        {/if}
      </Chat.BubbleAvatar>
      {#if message.role === 'assistant' && message.content === ''}
        <Chat.BubbleMessage typing={true} />
      {:else}
        <Chat.BubbleMessage class="text-foreground prose dark:prose-invert prose-sm max-w-auto p-3">
          {@html marked(sanitizeMarkdown(message.content))}
        </Chat.BubbleMessage>
      {/if}
    </Chat.Bubble>
  {/each}
</Chat.List>

{#if llmManager.error}
  <p class="error">{llmManager.error}</p>
{/if}
