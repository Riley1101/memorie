<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="" width="128" height="128">
</p>

<h1 align="center">Memoire</h1>

<p align="center">Writing at the speed of thought.</p>

Memoire is a local-first, privacy-focused writing and knowledge management app.
Your notes, files, and ideas live on your machine. An integrated assistant,
powered by a local large language model (LLM), lets you interrogate your own
writing without sending a word to the cloud.

## Features

- **Distraction-free canvas** — the smallest surface area we could defend; every panel earns its place.
- **Command palette** — type a colon, run anything.
- **Branching undo tree** — history that branches instead of collapsing into a straight line.
- **Local or hosted models** — run on-device, or point at a hosted model when you want one.
- **Assistant that waits to be asked** — suggestions you accept, or don't.
- **Binders & live outline** — structure long work, manuscript-ready in one sheet.
- **Goals, not streaks** — progress tracking without the guilt loop.
- **Git sync** — point at a GitHub repo; every save becomes a commit you own.
- **Dropbox sync** — point at a Dropbox folder; push and pull your writing on demand.
- **Export anywhere** — manuscript formats, no lock-in.

## Install

Grab the latest macOS build from [Releases](https://github.com/Riley1101/memorie/releases).
Once installed, Memoire checks for new versions in the background and offers a restart
when one is ready (or use **Settings → Updates**).

## Development

```sh
bun install
bun run tauri dev      # run the desktop app
bun run tauri build    # produce a release bundle
```

Dropbox sync needs an app key, which is baked in at build time:

```sh
DROPBOX_APP_KEY=your_app_key bun run tauri build
```

Create the app at [dropbox.com/developers/apps](https://www.dropbox.com/developers/apps)
with the `files.content.read`, `files.content.write` and `account_info.read` scopes, and
register `http://localhost:53682/` as a redirect URI. Login uses PKCE, so there is no
client secret to keep; the refresh token is stored in the OS keyring.

## Releasing

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


## Roadmap

A mobile companion and margin notes & review are planned.

A plugin API is being explored.
