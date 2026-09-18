# Contributing to Memoire

Thanks for looking. Memoire is a local-first writing app: a SvelteKit front end over a Rust
core, packaged with Tauri. It is MIT-licensed, and issues and pull requests are welcome.

The most useful contribution is usually a bug report with the file that caused it.

## Before you start

You need three things:

- **[Bun](https://bun.sh)** — the repo ships a `bun.lock`, so please use Bun rather than npm or pnpm.
- **[Rust](https://rustup.rs)** (stable, edition 2021) — `cargo` and `rustfmt` must be on your PATH.
- **Tauri 2 system dependencies** for your platform — follow
  [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites). On Linux this is the
  webkit2gtk / libsoup set; on macOS it is the Xcode command line tools.

## Running it

```sh
bun install
bun run tauri dev
```

That builds the Rust side and opens the desktop app with the front end hot-reloading.

`bun run dev` starts only the Vite server in a browser tab. Some of the app will not work there —
anything that talks to the filesystem, Git, or the local LLM goes through Tauri commands that do
not exist outside the desktop shell. Use it for pure UI work, not for testing behaviour.

`bun run dev:release` runs the desktop app with an optimised Rust build. It is slower to compile
and much faster at runtime, which matters when you are profiling editor responsiveness.

## Before you open a pull request

```sh
bun run format   # prettier over the front end, cargo fmt over src-tauri
bun run lint     # eslint
bun run check    # svelte-check against jsconfig.json
```

All three should be clean. `format` rewrites files in place, so run it first and commit the result.

There is no test suite yet. Until there is, say in the pull request what you actually exercised by
hand — which files you opened, what you typed, what you exported.

## How the code is laid out

```
src/
  routes/            SvelteKit pages: / (writings list), /[lexical] (editor),
                     /binders, /settings
  lib/components/    UI. editor.svelte and md-editor.svelte wrap Milkdown;
                     plugins/ holds the ProseMirror plugins (focus mode lives there)
  lib/runes/         Svelte 5 state classes — app, editor, fs, git, llm, tree, writing.
                     Shared state belongs here, not in component scope
  app.css            The design tokens. Colours are defined once, per theme

src-tauri/src/
  commands.rs        The command surface the front end calls into
  fs.rs, search.rs   Files on disk and the index over them
  git.rs             Git sync: GitHub auth, commit, push
  llm.rs, providers.rs, prompts.rs   Local and remote model plumbing
  export.rs, pdf.rs, rtf.rs          DOCX / EPUB / PDF output
  undotree.rs        The branching version history
```

Two conventions worth knowing before you write anything:

- **Colours come from `src/app.css`.** Use the semantic tokens (`bg-background`, `text-muted-foreground`,
  `border-border`). Do not hardcode hex values or reach for Tailwind's stock palette — themes are
  swapped by redefining those variables, and a literal colour survives the swap and looks wrong.
- **State lives in the runes classes.** If two components need to agree on something, put it in
  `src/lib/runes/`, not in a prop drilled through four layers.

## Style

Prettier and rustfmt decide formatting; do not argue with them in review. Beyond that: 100 column
lines, single quotes, semicolons — all of which `bun run format` applies for you.

Write comments that explain *why*, not *what*. The existing code does this and it is worth matching:
a comment saying a reference check avoids walking the whole document on every keystroke earns its
place; a comment saying a function sets a boolean does not.

## Commits and pull requests

Commit subjects in this repo are short and imperative — `Add backlink`, `Fix DOCX text boxes`,
`Serialize matrix jobs per platform in release workflow`. Match that. No prefix convention is
enforced.

Work on a branch and open a pull request against `main`; that is how everything here has landed so
far. Keep one pull request to one concern — a feature and a refactor in the same diff take an order
of magnitude longer to review.

In the description, cover what changed and why, and how you verified it by hand.

## Reporting bugs

Open an issue at [github.com/Riley1101/memorie/issues](https://github.com/Riley1101/memorie/issues)
with:

- your OS and app version (the version is in the status line, and in `src-tauri/tauri.conf.json`)
- what you expected and what happened instead
- the steps that get there from a fresh launch
- the Markdown file that triggered it, if it is safe to share — for anything involving export,
  import, or a crash on open, that file is the whole bug report

Memoire keeps your writing in plain Markdown on your own disk. Please redact before attaching, and
never paste an access token into an issue.

## Security

Do not open a public issue for a vulnerability. Report it privately to the maintainer through
GitHub instead — the repo handles GitHub tokens and file paths, and a public report is a working
exploit until it is patched.
