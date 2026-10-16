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

## Building a release bundle

```sh
bun run tauri build
```

### Dropbox app key

Dropbox sync needs an app key, which is baked in at build time:

```sh
DROPBOX_APP_KEY=your_app_key bun run tauri build
```

Create the app at [dropbox.com/developers/apps](https://www.dropbox.com/developers/apps)
with the `files.content.read`, `files.content.write` and `account_info.read` scopes, and
register `http://localhost:53682/` as a redirect URI. Login uses PKCE, so there is no
client secret to keep; the refresh token is stored in the OS keyring.

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

## Releasing (maintainers)

Bump the version everywhere it lives (`package.json`, `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`) and commit it with one command:

```sh
bun run release patch            # 0.3.0 → 0.3.1
bun run release minor            # 0.3.0 → 0.4.0
bun run release major            # 0.3.0 → 1.0.0
bun run release 0.5.0-beta.1     # an exact version
```

| Flag        | Effect                                                               |
| ----------- | -------------------------------------------------------------------- |
| `--dry-run` | Show what would change; write nothing                                |
| `--no-git`  | Edit the files but don't commit                                      |
| `--push`    | Also push this branch and fast-forward `release` to it, starting CI |

The script refuses to run with uncommitted changes, when the new version isn't higher
than the current one (installed apps would never update to it), or when the updater
public key is missing.

Pushing to the `release` branch runs `.github/workflows/release.yml`, which builds every
platform and creates a **draft** GitHub release with a `latest.json` for the updater.
Publish the draft — installed apps only see the update once it's published.

### Update signing (one-time setup)

Updates are signed, and the app refuses any update it can't verify.

```sh
bunx tauri signer generate -w ~/.tauri/memoire.key
```

- Put the contents of `~/.tauri/memoire.key.pub` in `plugins.updater.pubkey` in
  `src-tauri/tauri.conf.json`.
- Add the repo secrets `TAURI_SIGNING_PRIVATE_KEY` (contents of `~/.tauri/memoire.key`)
  and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

Keep the private key safe: if it's lost, existing installs can't receive updates
anymore.

## Security

Do not open a public issue for a vulnerability. Report it privately to the maintainer through
GitHub instead — the repo handles GitHub tokens and file paths, and a public report is a working
exploit until it is patched.
