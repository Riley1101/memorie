<script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import ChevrondownIcon from '@lucide/svelte/icons/chevron-down';
  import CheckIcon from '@lucide/svelte/icons/check';
  import SendIcon from '@lucide/svelte/icons/send';
  import BoldIcon from '@lucide/svelte/icons/bold';
  import ItalicIcon from '@lucide/svelte/icons/italic';
  import StrikethroughIcon from '@lucide/svelte/icons/strikethrough';
  import CodeIcon from '@lucide/svelte/icons/code';
  import { Textarea } from '$lib/components/ui/textarea/index.js';
  import { llmManager } from '@/runes/llm.svelte.js';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import { cn } from '$lib/utils';
  import { appState } from '$lib/runes/app.svelte.js';
  import { toggleMark } from '@milkdown/kit/prose/commands';

  /**
   * @typedef {Object} GrammarBoxProps
   * @property {string} originalText - The original text that needs grammar suggestions.
   * @property {(suggestion: string) => void} onFix - Callback function to apply the suggested fix.
   * @property {number} sequence - The sequence number of the text segment.
   * @property {import('prosemirror-view').EditorView} [view] - The ProseMirror view, for applying inline marks.
   */

  /** @type {GrammarBoxProps} */
  let { originalText, onFix, view } = $props();

  let customPrompt = $state('');

  const markButtons = [
    { label: 'Bold', markName: 'strong', icon: BoldIcon },
    { label: 'Italic', markName: 'emphasis', icon: ItalicIcon },
    { label: 'Strikethrough', markName: 'strike_through', icon: StrikethroughIcon },
    { label: 'Code', markName: 'inlineCode', icon: CodeIcon },
  ];

  /**
   * Toggle a mark on the editor's current selection. The widget can stay mounted
   * across selection changes within the same paragraph (ProseMirror dedupes the
   * decoration by key), so the live selection is read from the view at click time
   * rather than trusted from props.
   * @param {string} markName
   */
  function applyMark(markName) {
    if (!view) return;
    const markType = view.state.schema.marks[markName];
    if (!markType) return;
    toggleMark(markType)(view.state, view.dispatch);
    view.focus();
  }

  function handleAccept() {
    if (llmManager.editActionContent) {
      onFix(llmManager.editActionContent);
      llmManager.newEditActionSession();
    }
  }

  /**
   * @param {string} command
   */
  function handleSend(command) {
    let text = originalText;
    if (command === 'PromptExpansion') {
      text = `PROMPT: ${customPrompt} , Input Sentence: ${originalText}`;
    }
    llmManager.sendEditActionMessage(text, command);
  }

  const suggestionCategories = [
    {
      name: 'Polish',
      items: [
        { label: 'Polish grammar', command: 'CorrectGrammar' },
        { label: 'Improve clarity', command: 'ImproveClarity' },
        { label: 'Logical flow', command: 'LogicalFlow' },
      ]
    },
    {
      name: 'Tone',
      items: [
        { label: 'Make formal', command: 'MakeFormal' },
        { label: 'Make friendly', command: 'ToneFriendly' },
        { label: 'Make academic', command: 'ToneAcademic' },
      ]
    },
    {
      name: 'Length',
      items: [
        { label: 'Expand', command: 'LengthExpand' },
        { label: 'Shorten', command: 'LengthShorten' },
      ]
    },
    {
      name: 'Style',
      items: [
        { label: 'Simplify', command: 'Simplify' },
        { label: 'Creative', command: 'StyleCreative' },
      ]
    }
  ];
</script>

<div class="w-max max-w-sm flex flex-col gap-3 not-prose select-none animate-in fade-in slide-in-from-top-2 duration-300">
  
  {#if llmManager.editActionContent || llmManager.editActionInProgress}
    <div class="group relative bg-muted/40 backdrop-blur-sm border border-border/40 rounded-xl p-4 shadow-sm transition-all hover:shadow-md hover:bg-muted/50">
      <div class="flex items-start gap-4">
        <div class="mt-1 p-1.5 rounded-full bg-primary/10 text-primary">
          <SparklesIcon class={cn("size-4", llmManager.editActionInProgress ? "animate-pulse" : "")} />
        </div>
        
        <div class="flex-1 space-y-3">
          <div class="text-base leading-relaxed text-foreground font-writer tracking-tight">
            {#if llmManager.editActionContent}
              {llmManager.editActionContent}
            {:else}
              <span class="text-muted-foreground/60">Thinking...</span>
            {/if}
          </div>
          
          <div class="flex items-center gap-2">
            {#if llmManager.editActionContent && !llmManager.editActionInProgress}
              <Button 
                onclick={handleAccept} 
                size="sm" 
                disabled={false}
                class="h-8 px-4 bg-primary text-primary-foreground hover:bg-primary/90 rounded-full text-xs font-medium shadow-sm transition-all active:scale-95"
              >
                <CheckIcon class="size-3.5 mr-1.5" />
                Apply changes
              </Button>
            {/if}
            <Button 
              onclick={() => {
                if (llmManager.editActionInProgress) {
                  llmManager.cancelMessage();
                } else {
                  llmManager.newEditActionSession();
                }
              }} 
              variant="ghost" 
              size="sm" 
              disabled={false}
              class="h-8 px-3 text-muted-foreground hover:text-foreground rounded-full text-xs transition-colors"
            >
              Discard
            </Button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <div class="flex items-center gap-2">
    <div class="flex items-center gap-0.5 bg-background border border-border/40 rounded-full p-0.5 shadow-sm">
      {#each markButtons as { label, markName, icon: Icon } (markName)}
        <Button
          variant="ghost"
          size="icon"
          title={label}
          aria-label={label}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => applyMark(markName)}
          class="size-6 rounded-full text-muted-foreground/80 hover:text-foreground hover:bg-muted/50"
        >
          <Icon class="size-3.5" />
        </Button>
      {/each}
    </div>

    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        {#snippet child({ props })}
          <Button
            {...props}
            variant="ghost"
            size="sm"
            disabled={props.disabled ?? false}
            class="h-7 px-2.5 bg-background border border-border/40 hover:bg-muted/50 rounded-full text-[0.6875rem] font-medium tracking-wide text-muted-foreground/80 hover:text-foreground transition-all shadow-sm group"
          >
            <AiSparkleIcon class="size-3.5 mr-1.5 text-primary/60 group-hover:text-primary transition-colors" />
            AI Suggestions
            <ChevrondownIcon class="size-3 ml-1 opacity-40 group-hover:opacity-100 transition-opacity" />
          </Button>
        {/snippet}
      </DropdownMenu.Trigger>
      
      <DropdownMenu.Content portalProps={{}} class="w-72 {appState.ui.theme} p-2 border border-border/40 shadow-2xl rounded-xl backdrop-blur-xl bg-background/95" align="start">
        <div class="flex flex-col gap-2.5 p-1">
          <div class="relative group">
            <Textarea
              placeholder="Custom instructions..."
              class="min-h-[80px] text-xs resize-none bg-muted/20 border-border/20 focus:border-primary/30 focus:ring-primary/10 rounded-lg p-3 transition-all"
              bind:value={customPrompt}
            />
            <Button
              size="icon"
              variant="default"
              onclick={() => handleSend('PromptExpansion')}
              disabled={!customPrompt.trim()}
              class="absolute bottom-2 right-2 size-7 rounded-md shadow-sm transition-all active:scale-90"
            >
              <SendIcon class="size-3.5" />
            </Button>
          </div>
          
          <div class="h-px bg-border/20 mx-1"></div>
          
          <div class="max-h-[300px] overflow-y-auto pr-1">
            {#each suggestionCategories as category (category.name)}
              <div class="mt-2 first:mt-0">
                <div class="px-1 text-[0.625rem] font-semibold text-muted-foreground/50 uppercase tracking-widest pl-2 mb-1">
                  {category.name}
                </div>
                <div class="grid grid-cols-1 gap-0.5">
                  {#each category.items as item (item.command)}
                    <DropdownMenu.Item
                      closeOnSelect={true}
                      inset={false}
                      class="flex items-center px-2.5 py-2 text-xs rounded-lg cursor-pointer hover:bg-primary/10 hover:text-primary transition-colors focus:bg-primary/10 focus:text-primary outline-none group/item"
                      onclick={() => handleSend(item.command)}
                    >
                      <span class="flex-1">{item.label}</span>
                      <AiSparkleIcon class="size-3 opacity-0 group-hover/item:opacity-40 transition-opacity" />
                    </DropdownMenu.Item>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        </div>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  </div>
</div>
