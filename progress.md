# Memorie Glass — UI migration progress

The approved macOS-style redesign ("Memorie Glass") applied to every screen.
Mocks: design canvas https://claude.ai/artifact/7uzUUNXfSLrRHyvfiVEEx4 (boards named below).

Status key: `[ ]` to do · `[~]` in progress · `[x]` done · `[-]` deferred (reason given)

---

## 1. Design rules

**Three layers**
1. **Frame**: the window background, tinted (`--frame`). The sidebar floats on it as a glass panel, inset 8px, radius 12.
2. **Paper**: reading and writing happen on an opaque surface (`--paper`, the old `--background`).
3. **Glass**: used only for things that float. That means the dock, toolbar capsules, ⌘K palette, popovers and menus, note cards, sheets and dialogs, the assistant panel, and the Revise bar. The text selection toolbar uses dark glass.

**Type**
- UI text uses the system stack: `-apple-system, BlinkMacSystemFont, "Public Sans"`.
- UI text has fixed pixel sizes that do not scale with writing density: 13px base, 12px meta, 15px toolbar title.
- Section labels are 11px in the mocks, but use **12px**, to keep the 12px type floor that is already committed.
- Source Serif 4 is used only for writing, writing titles, codex entry names and quoted prose.

**Shapes**
- Capsules are fully round.
- Fields and popups: 8px.
- Groups, cards and the sidebar: 12px.
- Windows: 14px.
- Controls are 28–32px tall.

**Colour**
- There is one accent (#7d5129 light / #c99462 dark).
- Selected rows get a tinted fill: accent at 15% (light) or 36% (dark).
- A solid accent is used only for the primary action and the selected list item.

**Toolbar**
- 52px high. It is the window drag region.
- Traffic lights appear on macOS only, in the sidebar's top row (or at the toolbar's left edge when the sidebar is hidden).
- Title on the left on library pages. In the editor the title and subtitle are centred.
- Actions are grouped in glass capsules.

**Lists**: grouped inset lists in the style of System Settings (card, hairline separators, group header).

**Settings** uses its own pane layout: a sidebar of panes and one pane at a time.

**Linux / WebKitGTK**
- There is no real vibrancy, so the frame is a flat tint.
- Blur is kept to small floating elements, for performance.
- `prefers-reduced-transparency` makes glass opaque.

## 2. Decisions from canvas comments (must be honoured)

| # | Board | Decision |
|---|---|---|
| C1 | Home · Library | Rows have no excerpts or descriptions. The title is the first line and **binder · word count** sits under it. The time is on the right. |
| C2 | Home · Library | The "24 writings · 48,210 words" header subtitle is removed. The header shows only the title. |
| C3 | Home · Library | "Continue writing" is a **compact strip**: binder dot · `Continue` · title (serif) · "binder · edited 12 min ago" · `Resume`. It also appears in grid view. |
| C4 | Home · grid view | Cards show a page preview with the opening lines in serif, faded at the bottom, and an edge in the binder colour. Under the card: the title, then **only** the binder dot and binder name (**no word count, no date**). Groups are Today / Yesterday / Earlier. |
| C5 | Codex · Mentions | Per-entry mentions view showing:<br>• "N mentions in M writings" and the aliases that matched<br>• a by-chapter bar chart<br>• All / Linked / Unlinked chips and a Story order sort<br>• mentions grouped by writing, each sentence in serif with the name highlighted and its ¶ location<br>• "Possible mentions" with Ignore / Link<br>• a mention count on each row of the entry list |
| C6 | Main board | Dark is the default look of the board. Applies to the board only; the app follows the theme setting. |
| C7 | Home board | The hand-added note "where is the binder" is answered by C1: the binder name is on each row's second line. |

## 3. Inventory: every UI surface → board → phase

### Routes
| Surface | File | Board | Phase | Status |
|---|---|---|---|---|
| Root shell: frame, macOS drag strip, closing overlay, toaster | `src/routes/+layout.svelte` | all | 2 | [x] |
| Home: header, Continue strip, list/grid | `src/routes/+page.svelte` | Main, MainGrid | 3 | [x] |
| Editor: toolbar, history pane, scene panel | `src/routes/[lexical]/+page.svelte` | Editor | 3 | [x] |
| Editor layout: assistant panel host | `src/routes/[lexical]/+layout.svelte` | Chat, ChatEmpty | 3 | [x] |
| Binders | `src/routes/binders/+page.svelte` | Binders | 3 | [x] |
| Codex: sheets + mentions | `src/routes/codex/+page.svelte` | Codex, CodexMentions | 3 | [x] |
| Settings | `src/routes/settings/+page.svelte` | Settings | 4 | [x] |

### Shell and library components
| Surface | File | Board | Phase | Status |
|---|---|---|---|---|
| Floating sidebar: search, Library, Binders, Recently deleted, sync, settings | `binder-sidebar.svelte` | Main | 2 | [x] |
| Writing list / grid, groups | `timeline-documents.svelte` | Main, MainGrid | 3 | [x] |
| Binder tree rows | `binder-tree-node.svelte` | Main | 3 | [x] |
| Binder cards (binders page) | `binder-cards.svelte` | Binders | 3 | [x] |
| Scroll fades | `scroll-fade.svelte` | — | 3 | [x] |

### Overlays (portaled to body)
| Surface | File | Board | Phase | Status |
|---|---|---|---|---|
| ⌘K palette | `cmdk.svelte` | Palette | 3 | [x] |
| New writing picker | `new-writing-picker.svelte` | Palette | 3 | [x] |
| Project search (⌘⇧F) | `project-search.svelte` | Palette | 3 | [x] |
| Move picker | `move-picker.svelte` | Palette | 3 | [x] |
| Export sheet | `export-dialog.svelte` | Export, ExportMatter | 3 | [x] |
| Import dialog | `import-dialog.svelte` | Export (sheet style) | 3 | [x] |
| Recently deleted sheet | `trash-sheet.svelte` | Binders (inset list) | 3 | [x] |
| Shortcuts help | `shortcuts-help.svelte` | Settings (inset list) | 3 | [x] |
| Toasts | `+layout.svelte` Toaster | glass | 2 | [x] |

### Editor components
| Surface | File | Board | Phase | Status |
|---|---|---|---|---|
| Dock (word count, zoom, ⌘K) | `editor-commandbar.svelte`, `editor-stats.svelte` | Editor | 3 | [x] |
| Find bar | `editor-find.svelte` | Editor (glass capsule) | 3 | [x] |
| Revise bar + change popover | `revise-bar.svelte`, `editor.svelte` CSS | Editor | 3 | [x] |
| History pane + version graph | `editor-history.svelte`, `treeview/GitGraph.svelte` | Editor | 3 | [x] |
| Scene panel sections | `editor-scene-info`, `editor-notes`, `editor-codex`, `editor-outline`, `editor-backlinks` | Editor (inspector) | 3 | [x] |
| Margin notes + note card | `margin-notes.svelte`, `note-card.svelte` | Editor | 3 | [x] |
| Selection toolbar | `plugins/GrammarBox.svelte` | Editor (dark glass) | 3 | [x] |
| Link popover | `plugins/LinkPopover.svelte` | glass popover | 3 | [x] |
| Slash menu, doc-ref menu | `plugins/SlashMenu.svelte`, `plugins/DocRefMenu.svelte` | glass popover | 3 | [x] |
| Editor surface, title | `editor.svelte`, `md-editor.svelte` | Editor | 3 | [x] |

### Codex components
| Surface | File | Board | Phase | Status |
|---|---|---|---|---|
| Entry sheet | `codex-sheet.svelte` | Codex | 3 | [x] |
| Mentions | `codex-matrix.svelte` | CodexMentions | 3 | [x] |

### Assistant
| Surface | File | Board | Phase | Status |
|---|---|---|---|---|
| Assistant panel (was a drawer) | `ai-chat.svelte`, `[lexical]/+layout.svelte` | Chat | 3 | [x] |
| Conversation stream, sources, composer | `ai-chat-stream.svelte` | Chat, ChatEmpty | 3 | [x] |
| Searching state | `search-thoughts.svelte` | Chat | 3 | [x] |

### Shared primitives (`src/lib/components/ui/`)
| Surface | Phase | Status |
|---|---|---|
| dialog, sheet, drawer, popover, dropdown-menu, context-menu, tooltip, command, select → glass and the fixed radii | 1 | [x] |
| button, input, textarea, toggle, toggle-group → 28–32px controls, 8px fields, capsule buttons | 1 | [x] |

## 4. Phases

### Phase 1: Tokens and primitives
- [x] `--font-sans` becomes the system UI stack; fixed px UI sizes (`text-ui`, `text-ui-sm`, `text-title`).
- [x] Surface tokens `--frame`, `--paper`, `--glass`, `--hair`, `--fill`, `--selection`, light and dark.
- [x] Fixed radius tokens `rounded-field` (8), `rounded-group` (12), `rounded-window` (14), independent of the style flavour's `--radius`.
- [x] Component classes: `.glass` (refined), `.glass-dark`, `.capsule`, `.tb-btn`, `.seg-on`, `.inset-group`, `.sb-row`, `.sb-head`, `.field`, `.chip`.
- [x] Overlay primitives (dialog, sheet, popover, menus, tooltip, select, command) on glass.

### Phase 2: Window frame and sidebar
- [x] Root shell paints the frame; pages sit on paper.
- [x] Remove the opaque macOS title strip; each toolbar is the drag region; traffic-light row in the sidebar.
- [x] Floating glass sidebar on Home, Binders and Codex:
  - search with ⌘K
  - Library: All writings, Codex
  - Binders, with colour dots, counts and a + button
  - Recently deleted
  - sync status / Publish
  - Settings
- [x] Shared 52px toolbar pattern (title + capsules + primary).
- [x] Toasts on glass.

### Phase 3: Pages and overlays
- [x] Home list (C1, C2, C3) and grid (C3, C4).
- [x] Editor:
  - toolbar: back, sidebar toggle, centred title/subtitle, History + Scene capsule, Export
  - scene panel as a floating inspector
  - history pane
  - dock
  - find, Revise, margin notes
  - selection toolbar in dark glass
  - link, slash and doc-ref popovers
- [x] Binders page: grouped inset list.
- [x] Codex: three columns (sidebar, entry list grouped by kind, sheet); Mentions tab (C5).
- [x] Palette, new-writing picker, project search, move picker.
- [x] Export and import sheets, trash sheet, shortcuts help.
- [x] Assistant: floating 372px glass panel, empty state, sources, composer.

### Phase 4: Settings
- [x] Pane layout: Writing / Appearance / AI / Cloud sync / Storage / Updates, with grouped inset rows, toggles, segmented controls and popups.

### Phase 5: Audit
- [x] Walk every row of section 3 and confirm it uses the tokens (no leftover `font-writer` headings in chrome, no ad-hoc radii, no opaque floating panels).
- [ ] Keyboard: every shortcut still works; focus rings are visible. *(Needs a hands-on pass in the app.)*
- [ ] Margin notes, Revise, focus mode and draft mode still work. *(Needs a hands-on pass in the app.)*
- [x] `bun run check`, eslint and `bun run build` are clean; `cargo check` and the mentions test pass.

## 5. Log
- 2026-11-16: Plan written. Inventory taken from `src/routes` and `src/lib/components` (44 files plus the `ui/` primitives).
- 2026-11-17: Phases 1–2 done. Home (C1–C4), Binders, Codex with the per-entry Mentions view (C5, new `codex_mentions` backend command with a unit test), editor toolbar and floating panels, ⌘K palette.
  - C5 left out on purpose: the All / Linked / Unlinked filter and "Possible mentions" with Ignore / Link. The app has no model of a linked mention or an ignore list, so these are new features, not a restyle.
  - Needs checking on a Mac: `trafficLightPosition` (x 20, y 24) in `tauri.conf.json`, meant to centre the traffic lights in the 52px toolbar band. It cannot be seen from Linux.
- 2026-11-17 (later): Phase 3 overlays, editor plugins and assistant done; Phase 4 Settings done; Phase 5 automated audit clean.
  - Settings is a pane layout: a glass sidebar of panes (Writing, Appearance, AI, Cloud sync, Storage, Updates) and grouped inset rows. On/off choices are switches; multi-way choices are segmented controls. Storage is its own pane now (it used to sit under Appearance). Below 768px the panes wrap as chips.
  - `ScrollFade` fades with a mask instead of a paper gradient, so it works on glass. Its prop is now `fade` (px) instead of `fadeSize` (class).
  - The user bubble restyle moved to `ai-elements/message/core/message-content.svelte`, the component the stream actually renders. The unused `new-message` copy was reverted.
  - The find fields' error state was a border with no width, so it never showed. It is now a ring.
  - Note card and matrix tooltip are on glass.
  - Still open: the hands-on keyboard and editing pass, and the macOS traffic-light check.
