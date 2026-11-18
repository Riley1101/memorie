import { goto, invalidateAll } from '$app/navigation';
import { resolve } from '$app/paths';
import { invoke } from '@tauri-apps/api/core';
import { editorViewCtx } from '@milkdown/kit/core';
import AiSparkleIcon from '@lucide/svelte/icons/sparkles';
import BrainIcon from '@lucide/svelte/icons/brain';
import SaveIcon from '@lucide/svelte/icons/save';
import HistoryIcon from '@lucide/svelte/icons/history';
import TableOfContentsIcon from '@lucide/svelte/icons/table-of-contents';
import UndoIcon from '@lucide/svelte/icons/undo-2';
import RedoIcon from '@lucide/svelte/icons/redo-2';
import HouseIcon from '@lucide/svelte/icons/house';
import HelpIcon from '@lucide/svelte/icons/help-circle';
import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
import FocusIcon from '@lucide/svelte/icons/focus';
import NoteIcon from '@lucide/svelte/icons/message-square-text';
import SpellCheckIcon from '@lucide/svelte/icons/spell-check';
import FileOutputIcon from '@lucide/svelte/icons/file-output';
import QuoteIcon from '@lucide/svelte/icons/quote';
import BracketsIcon from '@lucide/svelte/icons/parentheses';
import { appState } from '$lib/runes/app.svelte.js';
import { editorState } from '$lib/runes/editor.svelte.js';
import { writingState } from '$lib/runes/writing.svelte.js';
import { llmManager } from '$lib/runes/llm.svelte.js';
import { configManager } from '$lib/runes/config.svelte.js';
import { memoryManager } from '$lib/runes/memory.svelte.js';
import { publish } from '$lib/runes/sync.svelte.js';
import { dirOf } from '$lib/runes/fs.svelte.js';
import { shortcut } from '$lib/keyboard.svelte.js';
import { toast } from '$lib/toast.js';
import { addNoteOnSelection } from '$lib/components/plugins/margin-notes.js';
import { compareWithStartOfToday, stopRevise } from '$lib/components/plugins/revise.js';
import { reviseState } from '$lib/runes/revise.svelte.js';
import CompareIcon from '@lucide/svelte/icons/git-compare';

/**
 * The writing open in the editor. The editor's dock sets it while it is
 * mounted, so the palette (which lives in the root layout) knows whether there
 * is a writing to act on, and which one.
 */
export const editorSession = $state({
  /** @type {string | null} */
  fileName: null,
  isDraft: false,
});

/** "Set up local AI" dialog, opened when the assistant is asked for before a model is ready. */
export const aiSetup = $state({ open: false });

/**
 * @typedef {Object} EditorCommand
 * @property {string} name - What used to be typed after the colon (":toc"). Still matched in the palette.
 * @property {string[]} [aliases] - Older names that still find it.
 * @property {string | (() => string)} label - What the palette shows, in plain words.
 * @property {string} [keywords] - Extra words to match on.
 * @property {'writing' | 'assistant' | 'typing'} group
 * @property {string} action
 * @property {string} [shortcutLabel]
 * @property {string} [key] - Key held with Meta/Ctrl. Empty = no shortcut here.
 * @property {boolean} [shift]
 * @property {boolean} [alt]
 * @property {boolean} [needsAi] - Only offered when AI is turned on.
 * @property {import('svelte').Component} icon
 */

/**
 * Shortcuts deliberately avoid keys the editor or the OS already own:
 * ⌘B/⌘I/⌘E (marks), ⌘Z/⌘⇧Z (editor undo/redo), ⌘Q (quit), ⌘H (hide), ⌘W (close window).
 * New writing, find in all writings, import and recently deleted are app-wide,
 * so the palette's own actions and the root layout's shortcuts own them.
 * @type {EditorCommand[]}
 */
const ALL_COMMANDS = [
  {
    name: 'focus',
    label: () => (writingState.focusMode ? 'Leave focus mode' : 'Focus mode'),
    keywords: 'distraction typewriter',
    group: 'writing',
    action: 'toggleFocus',
    shortcutLabel: shortcut('.'),
    key: '.',
    icon: FocusIcon,
  },
  {
    name: 'note',
    aliases: ['comment'],
    label: 'Leave a note on the selection',
    keywords: 'comment annotate margin',
    group: 'writing',
    action: 'note',
    shortcutLabel: shortcut('alt', 'M'),
    key: 'm',
    alt: true,
    icon: NoteIcon,
  },
  {
    name: 'toc',
    label: () => (appState.ui.isOutlineOpen ? 'Hide the scene panel' : 'Show the scene panel'),
    keywords: 'outline contents table',
    group: 'writing',
    action: 'toggleOutline',
    shortcutLabel: shortcut('shift', 'O'),
    key: 'o',
    shift: true,
    icon: TableOfContentsIcon,
  },
  {
    name: 'history',
    label: () => (appState.ui.isHistoryOpen ? 'Hide history' : 'Show history'),
    keywords: 'versions timeline',
    group: 'writing',
    action: 'toggleHistory',
    shortcutLabel: shortcut('U'),
    key: 'u',
    icon: HistoryIcon,
  },
  {
    name: 'revise',
    aliases: ['diff', 'compare'],
    label: () => (reviseState.open ? 'Stop comparing' : 'Compare with the start of today'),
    keywords: 'revise changes diff track since earlier version',
    group: 'writing',
    action: 'revise',
    shortcutLabel: shortcut('shift', 'R'),
    key: 'r',
    shift: true,
    icon: CompareIcon,
  },
  {
    name: 'u',
    label: 'Go to the previous version',
    keywords: 'undo back revert',
    group: 'writing',
    action: 'undo',
    shortcutLabel: shortcut('alt', 'Z'),
    key: 'z',
    alt: true,
    icon: UndoIcon,
  },
  {
    name: 'redo',
    label: 'Go to the next version',
    keywords: 'forward latest branch',
    group: 'writing',
    action: 'redo',
    shortcutLabel: shortcut('alt', 'shift', 'Z'),
    key: 'z',
    alt: true,
    shift: true,
    icon: RedoIcon,
  },
  {
    name: 'w',
    label: 'Save now',
    keywords: 'write',
    group: 'writing',
    action: 'save',
    shortcutLabel: shortcut('S'),
    key: 's',
    icon: SaveIcon,
  },
  {
    name: 'export',
    label: 'Export this writing…',
    keywords: 'word docx pdf epub manuscript compile',
    group: 'writing',
    action: 'export',
    shortcutLabel: shortcut('shift', 'E'),
    key: 'e',
    shift: true,
    icon: FileOutputIcon,
  },
  {
    name: 'sync',
    // "push" keeps the old name findable.
    aliases: ['push'],
    label: 'Publish to your cloud sync',
    keywords: 'git github dropbox upload',
    group: 'writing',
    action: 'push',
    icon: UploadCloudIcon,
  },
  {
    name: 'home',
    // ":b" was the old name, from when this toggled a sidebar; ":q" closed the writing.
    aliases: ['b', 'q'],
    label: 'Close and go home',
    keywords: 'close quit',
    group: 'writing',
    action: 'home',
    shortcutLabel: shortcut('shift', 'H'),
    key: 'h',
    shift: true,
    icon: HouseIcon,
  },
  {
    name: 'ai',
    label: 'Ask the assistant',
    keywords: 'chat ai help',
    group: 'assistant',
    action: 'assistant',
    shortcutLabel: shortcut('L'),
    key: 'l',
    needsAi: true,
    icon: AiSparkleIcon,
  },
  {
    name: 'c',
    label: 'Let the assistant read this writing',
    keywords: 'context memory index',
    group: 'assistant',
    action: 'createDocumentContext',
    needsAi: true,
    icon: BrainIcon,
  },
  {
    name: 'spell',
    label: () => (writingState.spellcheck ? 'Turn spellcheck off' : 'Turn spellcheck on'),
    group: 'typing',
    action: 'toggleSpellcheck',
    icon: SpellCheckIcon,
  },
  {
    name: 'smart',
    label: () =>
      writingState.smartPunctuation ? 'Turn smart punctuation off' : 'Turn smart punctuation on',
    keywords: 'curly quotes dashes typography',
    group: 'typing',
    action: 'toggleSmartPunctuation',
    icon: QuoteIcon,
  },
  {
    name: 'pairs',
    label: () =>
      writingState.autoPair ? 'Stop closing brackets for me' : 'Close brackets for me',
    keywords: 'auto pair brackets parentheses',
    group: 'typing',
    action: 'toggleAutoPair',
    icon: BracketsIcon,
  },
  {
    name: 'h',
    label: 'Keyboard shortcuts',
    keywords: 'help keys',
    group: 'typing',
    action: 'help',
    shortcutLabel: shortcut('/'),
    key: '/',
    icon: HelpIcon,
  },
];

/** Commands on offer right now: the assistant's only when AI is on. */
export function editorCommands() {
  const aiEnabled = configManager.config?.ai_enabled ?? false;
  return aiEnabled ? ALL_COMMANDS : ALL_COMMANDS.filter((c) => !c.needsAi);
}

/** @param {EditorCommand} command */
export function commandLabel(command) {
  return typeof command.label === 'function' ? command.label() : command.label;
}

/**
 * What the palette matches on: the words, plus the old ":name" so muscle
 * memory ("toc", ":export") still finds the command.
 * @param {EditorCommand} command
 */
export function commandSearchValue(command) {
  const names = [command.name, ...(command.aliases ?? [])].map((n) => `:${n}`).join(' ');
  return `${commandLabel(command)} ${names} ${command.keywords ?? ''}`;
}

/** Same path as autosave, so drafts get named and created the same way. */
async function saveNow() {
  if (editorState.flushSave) await editorState.flushSave();
}

function focusEditor() {
  editorState.editor?.action((ctx) => ctx.get(editorViewCtx).focus());
}

/**
 * Opens the assistant, or walks the writer through what it still needs:
 * an OpenRouter key, loading a downloaded model, or downloading one.
 */
export function openAssistant() {
  if (llmManager.isLoadModelsInProgress || llmManager.downloadingModelId) return;
  const provider = configManager.config?.provider ?? 'local';
  if (provider === 'openrouter') {
    if (!configManager.hasOpenRouterApiKey) {
      goto(resolve('/settings'));
      toast.info('Add your OpenRouter API key', 'Settings → AI → OpenRouter');
    } else {
      appState.toggleAiChat(!appState.ui.isChatOpen);
    }
    return;
  }
  if (!llmManager.modelsLoaded) {
    if (llmManager.modelStatuses.some((m) => m.downloaded)) {
      llmManager.loadModels();
    } else {
      aiSetup.open = true;
    }
    return;
  }
  appState.toggleAiChat(!appState.ui.isChatOpen);
}

/**
 * Runs a command against the open writing.
 * @param {EditorCommand} command
 */
export async function runEditorCommand(command) {
  const { fileName, isDraft } = editorSession;
  if (fileName === null) return;

  // Drafts have no versions yet and nothing to push.
  if (isDraft && ['undo', 'redo', 'toggleHistory', 'push', 'revise'].includes(command.action)) {
    toast.info('Start writing first', 'Versions appear after the first save.');
    return;
  }

  switch (command.action) {
    case 'note':
      addNoteOnSelection();
      break;
    case 'createDocumentContext':
      memoryManager.createDocumentContext();
      break;
    case 'revise':
      if (reviseState.open) stopRevise();
      else await compareWithStartOfToday(fileName);
      break;
    case 'toggleHistory':
      appState.toggleChatHistory();
      break;
    case 'toggleOutline':
      appState.toggleOutline();
      break;
    case 'toggleFocus':
      writingState.toggleFocusMode();
      focusEditor();
      break;
    case 'export':
      appState.openExport({ fileName: isDraft ? null : fileName, dir: dirOf(fileName) });
      break;
    case 'toggleSpellcheck':
      writingState.setSpellcheck(!writingState.spellcheck);
      toast.info(writingState.spellcheck ? 'Spellcheck on' : 'Spellcheck off');
      break;
    case 'toggleSmartPunctuation':
      writingState.setSmartPunctuation(!writingState.smartPunctuation);
      toast.info(writingState.smartPunctuation ? 'Smart punctuation on' : 'Smart punctuation off');
      break;
    case 'toggleAutoPair':
      writingState.setAutoPair(!writingState.autoPair);
      toast.info(writingState.autoPair ? 'Auto-closing brackets on' : 'Auto-closing brackets off');
      break;
    case 'home':
      // Leaving the document: make sure the last words are on disk first.
      await saveNow();
      goto(resolve('/'));
      break;
    case 'assistant':
      openAssistant();
      break;
    case 'save':
      await saveNow();
      break;
    case 'push':
      // Save first so the open writing's latest edits go out too.
      await saveNow();
      await publish();
      break;
    case 'undo':
    case 'redo': {
      const older = command.action === 'undo';
      try {
        const moved = await invoke(older ? 'undo_file' : 'redo_file', { name: fileName });
        if (moved === null || moved === undefined) {
          toast.info(older ? 'Already at the oldest version' : 'Already at the latest version');
          break;
        }
        await invalidateAll();
        appState.incrementEditorVersion();
      } catch (e) {
        console.error(older ? 'Undo failed:' : 'Redo failed:', e);
        toast.error(older ? 'Could not go to previous version' : 'Could not go to next version', e);
      }
      break;
    }
    case 'help':
      appState.toggleHelpModal(!appState.ui.isHelpModalOpen);
      break;
  }
}
