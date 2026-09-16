#!/usr/bin/env bash
# Strips display-stack libraries that Tauri's AppImage bundler (linuxdeploy's
# gtk plugin) pulls in from the CI's Ubuntu 22.04 image. These libraries
# shadow the host's own copies at runtime and make newer Mesa builds fail to
# negotiate EGL (EGL_BAD_PARAMETER) on Wayland, notably on Arch, Fedora,
# NixOS and openSUSE. See tauri-apps/tauri#15976.
#
# Tauri has no built-in way to exclude libraries from the AppImage yet
# (bundle.linux.appimage.excludeLibraries is still an open PR as of tauri
# 2.11), so this repackages the AppImage after the fact: extract, delete the
# offending libs so the AppImage's AppRun falls back to the host's versions,
# then rebuild the squashfs image with appimagetool.
set -euo pipefail

EXCLUDE_PATTERNS=(
  'libwayland-client.so*'
  'libwayland-cursor.so*'
  'libwayland-egl.so*'
  'libwayland-server.so*'
  'libxkbcommon.so*'
  'libxcb-randr.so*'
  'libxcb-render.so*'
  'libxcb-shm.so*'
  'libXau.so*'
  'libXdmcp.so*'
)

APPIMAGETOOL_URL_BASE="https://github.com/AppImage/appimagetool/releases/download/continuous"

mapfile -t appimages < <(find src-tauri/target -type f -path '*/bundle/appimage/*.AppImage' 2>/dev/null)

if [[ ${#appimages[@]} -eq 0 ]]; then
  echo "strip-appimage-egl-libs: no AppImage found, nothing to do."
  exit 0
fi

arch="$(uname -m)"
tool_dir="$(mktemp -d)"
trap 'rm -rf "$tool_dir"' EXIT

appimagetool="$tool_dir/appimagetool-$arch.AppImage"
curl -fsSL -o "$appimagetool" "$APPIMAGETOOL_URL_BASE/appimagetool-$arch.AppImage"
chmod +x "$appimagetool"

for appimage in "${appimages[@]}"; do
  echo "strip-appimage-egl-libs: processing $appimage"
  work_dir="$(mktemp -d)"

  cp "$appimage" "$work_dir/orig.AppImage"
  chmod +x "$work_dir/orig.AppImage"
  (cd "$work_dir" && APPIMAGE_EXTRACT_AND_RUN=1 ./orig.AppImage --appimage-extract >/dev/null)

  removed=0
  for pattern in "${EXCLUDE_PATTERNS[@]}"; do
    while IFS= read -r -d '' lib; do
      rm -f "$lib"
      echo "  removed ${lib#"$work_dir"/squashfs-root/}"
      removed=$((removed + 1))
    done < <(find "$work_dir/squashfs-root" -type f -name "$pattern" -print0)
  done

  if [[ "$removed" -eq 0 ]]; then
    echo "  no matching libraries bundled, leaving AppImage untouched."
    rm -rf "$work_dir"
    continue
  fi

  rm -f "$appimage"
  ARCH="$arch" APPIMAGE_EXTRACT_AND_RUN=1 "$appimagetool" --appimage-extract-and-run "$work_dir/squashfs-root" "$appimage" >/dev/null
  chmod +x "$appimage"
  rm -rf "$work_dir"
done
