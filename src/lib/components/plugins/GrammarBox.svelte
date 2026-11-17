<script>
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
  import ChevrondownIcon from '@lucide/svelte/icons/chevron-down';
  import CheckIcon from '@lucide/svelte/icons/check';
  import SendIcon from '@lucide/svelte/icons/send';
  import BoldIcon from '@lucide/svelte/icons/bold';
  import ItalicIcon from '@lucide/svelte/icons/italic';
  import LinkIcon from '@lucide/svelte/icons/link';
  import NoteIcon from '@lucide/svelte/icons/message-square-text';
  import { editLink } from './link-popover.svelte.js';
  import { addNoteOnSelection } from './margin-notes.js';
  import { shortcut } from '$lib/keyboard.svelte.js';
  import { Textarea } from '$lib/components/ui/textarea/index.js';
  import { llmManager } from '@/runes/llm.svelte.js';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import { cn } from '$lib/utils';
  import { appState } from '$lib/runes/app.svelte.js';
  import { configManager } from '$lib/runes/config.svelte.js';
  import { toggleMark } from '@milkdown/kit/prose/commands';
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import { page } from '$app/state';
  import { editorState } from '$lib/runes/editor.svelte.js';
  import { openCodex } from '$lib/runes/codex.svelte.js';
  import { toast } from '$lib/toast.js';

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

  /** The formatting buttons always show; the AI parts only when AI is on. */
  let aiEnabled = $derived(Boolean(configManager.config?.ai_enabled));

  /**
   * Follows the text as it streams in, so the newest words stay visible once
   * the suggestion outgrows its max height. Re-runs whenever the content changes.
   * @param {HTMLElement} el
   */
  function followStream(el) {
    llmManager.editActionContent;
    if (llmManager.editActionInProgress) {
      el.scrollTop = el.scrollHeight;
    }
  }

  /**
   * Bold and italic are what prose uses; strikethrough and code are still in
   * the slash menu and Markdown for the writers who want them.
   */
  const markButtons = [
    { label: 'Bold', markName: 'strong', icon: BoldIcon, keys: shortcut('B') },
    { label: 'Italic', markName: 'emphasis', icon: ItalicIcon, keys: shortcut('I') },
  ];

  const toolButton =
    'flex size-8 items-center justify-center rounded-lg text-background/80 transition-colors hover:bg-background/15 hover:text-background focus-visible:outline-2 focus-visible:outline-background/60';
  const toolLabel =
    'flex h-8 items-center gap-1.5 rounded-lg px-2.5 text-[0.8125rem] font-medium text-background/90 transition-colors hover:bg-background/15 hover:text-background focus-visible:outline-2 focus-visible:outline-background/60';

  function addLink() {
    if (view) editLink(view);
  }

  function addNote() {
    addNoteOnSelection();
  }

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

  /** Longest selection offered as a codex entry's name. */
  const CODEX_NAME_MAX = 60;

  /** The selection, if it's short enough to be a name. Read at click time, like `applyMark`. */
  function selectedName() {
    if (!view) return '';
    const { from, to } = view.state.selection;
    const text = view.state.doc.textBetween(from, to, ' ', ' ').replace(/\s+/g, ' ').trim();
    return text && text.length <= CODEX_NAME_MAX ? text.replace(/['’]s$/, '') : '';
  }

  async function addToCodex() {
    const name = selectedName();
    if (!name) {
      toast.info('Select just a name to add it to the codex');
      return;
    }
    const current = page.params.lexical ?? '';
    await editorState.flushSave?.();
    openCodex({ binder: current.includes('/') ? current.split('/')[0] : '', new: 'character', name });
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

<div class="w-max max-w-md flex flex-col gap-2 not-prose select-none animate-in fade-in duration-150 motion-reduce:animate-none group-data-[placement=below]/box:flex-col-reverse">
  
  {#if aiEnabled && (llmManager.editActionContent || llmManager.editActionInProgress)}
    <div class="group relative bg-muted/50 rounded-xl p-4 transition-colors hover:bg-muted/70">
      <div class="flex items-start gap-4">
        <div class="mt-1 p-1.5 rounded-full bg-primary/10 text-primary">
          <SparklesIcon strokeWidth={1.5} class={cn("size-4", llmManager.editActionInProgress ? "animate-pulse" : "")} />
        </div>
        
        <div class="flex-1 min-w-0 space-y-3">
          <!-- Long suggestions scroll here so Apply/Discard stay in view. -->
          <div
            {@attach followStream}
            class="max-h-64 overflow-y-auto overscroll-contain pr-1 text-base leading-relaxed text-foreground font-writer tracking-tight"
          >
            {#if llmManager.editActionContent}
              {llmManager.editActionContent}
            {:else}
              <span class="text-muted-foreground">Thinking…</span>
            {/if}
          </div>
          
          <div class="flex items-center gap-2">
            {#if llmManager.editActionContent && !llmManager.editActionInProgress}
              <Button 
                onmousedown={(e) => e.preventDefault()}
                onclick={handleAccept} 
                size="sm" 
                disabled={false}
                class="h-8 px-4 bg-primary text-primary-foreground hover:bg-primary/90 rounded-full text-xs font-medium transition-[background-color,transform] active:scale-95"
              >
                <CheckIcon strokeWidth={1.5} class="size-3.5 mr-1.5" />
                Apply changes
              </Button>
            {/if}
            <Button 
              onmousedown={(e) => e.preventDefault()}
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

  <!-- One dark pill, like a printer's proof mark: the same in every theme. -->
  <div
    role="toolbar"
    aria-label="Selection"
    class="flex items-center gap-0.5 self-start rounded-xl bg-foreground p-1 text-background shadow-lg"
  >
    {#each markButtons as { label, markName, icon: Icon, keys } (markName)}
      <button
        type="button"
        title="{label} ({keys})"
        aria-label={label}
        onmousedown={(e) => e.preventDefault()}
        onclick={() => applyMark(markName)}
        class={toolButton}
      >
        <Icon strokeWidth={1.75} class="size-4" />
      </button>
    {/each}
    <button
      type="button"
      title="Link ({shortcut('K')})"
      aria-label="Link"
      onmousedown={(e) => e.preventDefault()}
      onclick={addLink}
      class={toolButton}
    >
      <LinkIcon strokeWidth={1.75} class="size-4" />
    </button>
    <span class="mx-1 h-4 w-px bg-background/20" aria-hidden="true"></span>
    <button
      type="button"
      title="Leave a note ({shortcut('alt', 'M')})"
      onmousedown={(e) => e.preventDefault()}
      onclick={addNote}
      class={toolLabel}
    >
      <NoteIcon strokeWidth={1.75} class="size-4" />
      <span class="hidden sm:inline">Comment</span>
    </button>
    <button
      type="button"
      title="Add to codex"
      aria-label="Add to codex"
      onmousedown={(e) => e.preventDefault()}
      onclick={addToCodex}
      class={toolLabel}
    >
      <BookUserIcon strokeWidth={1.75} class="size-4" />
      <span class="hidden sm:inline">Codex</span>
    </button>

    {#if aiEnabled}
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <button
              {...props}
              type="button"
              onmousedown={(e) => e.preventDefault()}
              class={toolLabel}
            >
              <AiSparkleIcon strokeWidth={1.75} class="size-4" />
              <span class="hidden sm:inline">Rewrite</span>
              <ChevrondownIcon strokeWidth={1.75} class="size-3.5 opacity-60" />
            </button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content portalProps={{}} class="w-72 {appState.ui.theme} p-2 border border-border/40 shadow-lg rounded-xl bg-popover" align="start">
          <div class="flex flex-col gap-2.5 p-1">
            <div class="relative group">
              <Textarea
                placeholder="Custom instructions..."
                class="min-h-[80px] text-xs resize-none bg-muted/20 border-border/20 focus:border-primary/30 focus:ring-primary/10 rounded-lg p-3 transition-colors"
                bind:value={customPrompt}
   />
              <Button
                size="icon"
                variant="default"
                onclick={() => handleSend('PromptExpansion')}
                disabled={!customPrompt.trim()}
                class="absolute bottom-2 right-2 size-7 rounded-md transition-[background-color,transform] active:scale-90"
              >
                <SendIcon strokeWidth={1.5} class="size-3.5" />
              </Button>
            </div>
          
            <div class="h-px bg-border/20 mx-1"></div>
          
            <div class="max-h-[300px] overflow-y-auto pr-1">
              {#each suggestionCategories as category (category.name)}
                <div class="mt-2 first:mt-0">
                  <div class="px-1 text-xs font-medium text-muted-foreground pl-2 mb-1">
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
                        <AiSparkleIcon strokeWidth={1.5} class="size-3 opacity-0 group-hover/item:opacity-40 transition-opacity" />
                      </DropdownMenu.Item>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    {/if}
  </div>
</div>
