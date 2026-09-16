#!/usr/bin/env bash
# Drop-in replacement for tauri-action's default build invocation (see the
# `tauriScript` input in .github/workflows/release.yml). Runs the normal
# `tauri` CLI with whatever arguments tauri-action passes, then on a Linux
# `build` run, strips the display-stack libraries that break AppImage
# launches on newer host distros (see strip-appimage-egl-libs.sh).
set -euo pipefail

bun run tauri "$@"

if [[ "${1:-}" == "build" ]]; then
  "$(dirname "${BASH_SOURCE[0]}")/strip-appimage-egl-libs.sh"
fi
