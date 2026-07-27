#!/bin/sh
# signal installer — https://ianaya89.github.io/signal/
#
#   curl -fsSL https://ianaya89.github.io/signal/install.sh | sh
#
# Downloads the latest release asset for this platform and installs it:
#   macOS  → mounts the .dmg and copies signal.app to /Applications
#   Linux  → .deb if apt is available, otherwise the AppImage into ~/.local/bin
#
# POSIX sh on purpose: this runs on whatever the user has.
set -eu

REPO="ianaya89/signal"
API="https://api.github.com/repos/${REPO}/releases/latest"

BOLD=''; DIM=''; RED=''; RESET=''
if [ -t 1 ]; then
  BOLD="$(printf '\033[1m')"; DIM="$(printf '\033[2m')"
  RED="$(printf '\033[31m')"; RESET="$(printf '\033[0m')"
fi

say() { printf '%s▶%s %s\n' "$BOLD" "$RESET" "$1"; }
die() { printf '%s✕%s %s\n' "$RED" "$RESET" "$1" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but not installed"; }
need curl

OS="$(uname -s)"
ARCH="$(uname -m)"

# ------------------------------------------------------------------ release
say "looking up the latest release"
JSON="$(curl -fsSL "$API" 2>/dev/null || true)"
[ -n "$JSON" ] || die "no published release yet.

signal is pre-release: build it from source instead —
  git clone https://github.com/${REPO} && cd signal
  mise trust && mise install && pnpm install
  brew install pkgconf mpv     # or: apt install libmpv-dev
  pnpm tauri dev"

TAG="$(printf '%s' "$JSON" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"

# Picks the first asset URL whose name matches the pattern.
asset_url() {
  printf '%s' "$JSON" |
    tr ',' '\n' |
    grep '"browser_download_url"' |
    sed -n 's/.*"browser_download_url": *"\([^"]*\)".*/\1/p' |
    grep -i "$1" |
    head -1
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

case "$OS" in
# -------------------------------------------------------------------- macOS
Darwin)
  case "$ARCH" in
    arm64) PATTERN='arm64\.dmg$' ;;
    x86_64) PATTERN='x86_64\.dmg$' ;;
    *) die "unsupported macOS architecture: $ARCH" ;;
  esac
  URL="$(asset_url "$PATTERN")"
  [ -n "$URL" ] || die "release ${TAG} has no .dmg for ${ARCH} — see https://github.com/${REPO}/releases"

  say "downloading ${DIM}$(basename "$URL")${RESET}"
  curl -fL# -o "$TMP/signal.dmg" "$URL"

  say "mounting"
  MNT="$(mktemp -d)"
  hdiutil attach -nobrowse -quiet -mountpoint "$MNT" "$TMP/signal.dmg"

  # Releases up to v0.1.0 shipped the bundle as Signal.app, later ones as
  # signal.app — take whatever the dmg actually contains.
  SRC="$(find "$MNT" -maxdepth 1 -name '*.app' | head -1)"
  [ -n "$SRC" ] || die "no .app inside the dmg"
  APP="/Applications/$(basename "$SRC")"

  # Replace any previous install rather than merging two app bundles.
  for prev in /Applications/Signal.app /Applications/signal.app; do
    [ -d "$prev" ] || continue
    say "removing the previous ${prev}"
    rm -rf "$prev"
  done
  say "copying to /Applications"
  cp -R "$SRC" /Applications/
  hdiutil detach -quiet "$MNT" || true
  rmdir "$MNT" 2>/dev/null || true

  # The build is ad-hoc signed, so Gatekeeper would otherwise refuse it.
  xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true

  say "installed ${BOLD}signal ${TAG}${RESET} → ${APP}"
  echo "  open it with: open -a $(basename "$APP" .app)"
  ;;

# -------------------------------------------------------------------- Linux
Linux)
  [ "$ARCH" = "x86_64" ] || die "only x86_64 Linux builds are published — build from source for ${ARCH}"

  # Prefer the .deb when apt can actually run it: it pulls libmpv from the
  # distro instead of carrying a second copy inside an AppImage.
  if command -v apt-get >/dev/null 2>&1; then
    if [ "$(id -u)" -eq 0 ] || command -v sudo >/dev/null 2>&1; then
      URL="$(asset_url 'amd64\.deb$')"
    fi
  fi
  if [ -n "${URL:-}" ]; then
    say "downloading ${DIM}$(basename "$URL")${RESET}"
    curl -fL# -o "$TMP/signal.deb" "$URL"
    say "installing with apt (pulls in libmpv)"
    if [ "$(id -u)" -eq 0 ]; then SUDO=""; else SUDO="sudo"; fi
    if $SUDO apt-get install -y "$TMP/signal.deb"; then
      say "installed ${BOLD}signal ${TAG}${RESET} — launch it from your desktop menu"
      exit 0
    fi
    # The .deb needs libmpv2, i.e. Ubuntu 24.04+ / Debian 13+. Older releases
    # ship libmpv1 and get the self-contained AppImage instead.
    say "apt declined the .deb — falling back to the AppImage"
  fi

  URL="$(asset_url 'amd64\.AppImage$')"
  [ -n "$URL" ] || die "release ${TAG} has no Linux asset — see https://github.com/${REPO}/releases"

  # Installed as signal-app: the name `signal` belongs to the CLI.
  DEST="${XDG_BIN_HOME:-$HOME/.local/bin}"
  mkdir -p "$DEST"
  say "downloading ${DIM}$(basename "$URL")${RESET}"
  curl -fL# -o "$DEST/signal-app" "$URL"
  chmod +x "$DEST/signal-app"

  say "installed ${BOLD}signal ${TAG}${RESET} → ${DEST}/signal-app"
  case ":$PATH:" in
    *":$DEST:"*) echo "  run it with: signal-app" ;;
    *) echo "  ${DEST} is not on your PATH — add it, or run ${DEST}/signal-app" ;;
  esac
  ;;

*)
  die "unsupported platform: ${OS}. Windows is not built yet — see https://github.com/${REPO}"
  ;;
esac
