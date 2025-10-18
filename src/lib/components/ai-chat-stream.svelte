<script>
    import {llmManager} from "@/runes/llm.svelte.js";
    import * as Chat from "$lib/components/ui/chat";
    import {onDestroy, onMount} from "svelte";

    onMount(() => {
        llmManager.setupListeners();
    })

    onDestroy(() => {
        llmManager.destroy();
    })

</script>

<Chat.List>
    {#each llmManager.messages as message}
        <Chat.Bubble variant={message.role === 'user' ? 'sent' : 'received'}>
            <Chat.BubbleAvatar>
                <Chat.BubbleAvatarImage
                        src="https://github.com/shadcn.png"
                        alt="@shadcn"
                />
                <Chat.BubbleAvatarFallback>
                    {message.role}
                </Chat.BubbleAvatarFallback>
            </Chat.BubbleAvatar>
            {#if message.role === 'assistant' && message.content === ""}
                <Chat.BubbleMessage typing={true}/>
            {:else}
                <Chat.BubbleMessage class="prose markdown dark:prose-invert">
                    {message.content}
                </Chat.BubbleMessage>
            {/if}
        </Chat.Bubble>
    {/each}
</Chat.List>

{#if llmManager.error}
    <p class="error">{llmManager.error}</p>
{/if}