#!/usr/bin/env bash
# Local macOS release. GitHub-hosted macOS runners cost 10x Linux minutes, so
# the .dmg is built here and the Linux artifacts come from
# .github/workflows/release.yml — both upload to the same draft release.
#
# The dmg is self-contained: libmpv and its ffmpeg/libass tree are copied into
# signal.app/Contents/Frameworks and the install names rewritten, so it runs on
# Macs without Homebrew.
#
#   ./scripts/release-local.sh 0.2.0            # build only
#   ./scripts/release-local.sh 0.2.0 --publish  # ...and upload to the draft release
#
# Signing: ad-hoc by default. Export APPLE_SIGNING_IDENTITY="Developer ID
# Application: …" to sign properly (and notarize separately).
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:-}"
shift || true
PUBLISH=0
SKIP_TAG_CHECK=0
for arg in "$@"; do
  case "$arg" in
    --publish) PUBLISH=1 ;;
    --skip-tag-check) SKIP_TAG_CHECK=1 ;;
    *) echo "unknown flag: $arg" >&2; exit 1 ;;
  esac
done

[ -n "$VERSION" ] || {
  echo "usage: ./scripts/release-local.sh <version> [--publish] [--skip-tag-check]" >&2
  exit 1
}
[ "$(uname -s)" = "Darwin" ] || { echo "error: this script builds the macOS artifacts only" >&2; exit 1; }

TAG="v${VERSION}"
ARCH="$(uname -m)"          # arm64 | x86_64 — libmpv comes from Homebrew, which
                            # is single-arch, so no universal build here.
APP_NAME="signal"
BUNDLE_DIR="target/release/bundle/macos"
APP="${BUNDLE_DIR}/${APP_NAME}.app"
DIST="dist-release"
DMG="${DIST}/${APP_NAME}_${VERSION}_${ARCH}.dmg"

say() { printf '\n\033[1;35m▶\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m✕\033[0m %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------- preflight
say "preflight"
for tool in pnpm cargo pkg-config hdiutil codesign; do
  command -v "$tool" >/dev/null || die "$tool not found in PATH"
done
pkg-config --exists mpv || die "libmpv not found by pkg-config — brew install pkgconf mpv"
if ! command -v dylibbundler >/dev/null; then
  echo "dylibbundler missing — installing (needed to make the .app self-contained)"
  brew install dylibbundler || die "brew install dylibbundler failed"
fi
[ "$PUBLISH" -eq 0 ] || command -v gh >/dev/null || die "gh not found (needed for --publish)"

if [ "$SKIP_TAG_CHECK" -eq 0 ]; then
  git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null 2>&1 || die \
    "tag ${TAG} not found. Create and push it first:
    git tag ${TAG} && git push origin ${TAG}
  (or pass --skip-tag-check for a test build)"
  if [ "$(git rev-parse HEAD)" != "$(git rev-parse "${TAG}^{commit}")" ]; then
    echo "warning: HEAD is not at ${TAG} — building HEAD, not the tagged commit."
  fi
fi

# ------------------------------------------------------------------- build
say "writing version ${VERSION} into the manifests"
./scripts/set-version.sh "$VERSION"

say "installing frontend deps"
pnpm install --frozen-lockfile

say "building ${APP_NAME}.app (${ARCH})"
# Wiped first because APFS is case-insensitive but case-preserving: a bundle dir
# left over from the old "Signal.app" name would keep its capital S forever.
rm -rf "$BUNDLE_DIR"
# Only the .app here: the dmg is assembled after libmpv is bundled in, since
# rewriting install names would invalidate a dmg built in the same pass.
pnpm tauri build --bundles app
[ -d "$APP" ] || die "expected app bundle at ${APP}"

EXE="${APP}/Contents/MacOS/${APP_NAME}"
[ -x "$EXE" ] || EXE="$(find "${APP}/Contents/MacOS" -maxdepth 1 -type f -perm -111 | head -1)"
[ -n "$EXE" ] || die "no executable inside ${APP}"

# --------------------------------------------------------------- bundle mpv
say "bundling libmpv and its dependency tree"
dylibbundler \
  --overwrite-files --bundle-deps --create-dir \
  --fix-file "$EXE" \
  --dest-dir "${APP}/Contents/Frameworks" \
  --install-path '@executable_path/../Frameworks' \
  --search-path /opt/homebrew/lib \
  --search-path /usr/local/lib

# dyld on macOS 15+ aborts at launch when a Mach-O carries the same LC_RPATH
# twice, and dylibbundler adds @executable_path/../Frameworks to libs that were
# already built with it (libmpv is one). Collapse the repeats.
say "de-duplicating LC_RPATH entries"
dedupe_rpaths() {
  local file="$1" path count
  while read -r path count; do
    [ -n "$path" ] || continue
    while [ "$count" -gt 1 ]; do
      install_name_tool -delete_rpath "$path" "$file" 2>/dev/null
      count=$((count - 1))
    done
    echo "  ${file##*/}: collapsed duplicate rpath ${path}"
  done < <(otool -l "$file" | awk '/cmd LC_RPATH/{getline;getline;print $2}' |
    sort | uniq -cd | awk '{print $2, $1}')
}
dedupe_rpaths "$EXE"
while IFS= read -r dylib; do
  dedupe_rpaths "$dylib"
done < <(find "${APP}/Contents/Frameworks" -name '*.dylib')

# A leftover absolute Homebrew path means the app only runs on this machine.
if otool -L "$EXE" | grep -qE '/(opt/homebrew|usr/local)/'; then
  otool -L "$EXE" | grep -E '/(opt/homebrew|usr/local)/' >&2
  die "executable still links Homebrew paths — the dmg would not run elsewhere"
fi
FRAMEWORK_COUNT=$(find "${APP}/Contents/Frameworks" -name '*.dylib' | wc -l | tr -d ' ')
echo "bundled ${FRAMEWORK_COUNT} dylibs"

# ---------------------------------------------------------------- codesign
IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
if [ "$IDENTITY" = "-" ]; then
  say "ad-hoc signing (no APPLE_SIGNING_IDENTITY set)"
  # No secure timestamp and no hardened runtime: an ad-hoc signature cannot be
  # notarized anyway, and hardened runtime only adds failure modes here.
  SIGN_FLAGS=(--timestamp=none)
else
  say "signing with: ${IDENTITY}"
  # Hardened runtime + secure timestamp are both required for notarization.
  SIGN_FLAGS=(--timestamp --options runtime)
fi

# install_name_tool invalidated every signature, so re-sign inside out.
find "${APP}/Contents/Frameworks" -name '*.dylib' -print0 |
  xargs -0 -I{} codesign --force "${SIGN_FLAGS[@]}" --sign "$IDENTITY" {}
codesign --force --deep "${SIGN_FLAGS[@]}" --sign "$IDENTITY" "$APP"
codesign --verify --deep --strict "$APP" || die "codesign verification failed"

# --------------------------------------------------------------------- dmg
say "assembling ${DMG}"
rm -rf "$DIST" "${TMPDIR:-/tmp}/signal-dmg"
mkdir -p "$DIST" "${TMPDIR:-/tmp}/signal-dmg"
STAGE="${TMPDIR:-/tmp}/signal-dmg"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "${APP_NAME} ${VERSION}" -srcfolder "$STAGE" \
  -ov -format UDZO -quiet "$DMG"
rm -rf "$STAGE"

shasum -a 256 "$DMG" | tee "${DMG}.sha256"
echo
ls -lh "$DMG"

# ----------------------------------------------------------------- publish
if [ "$PUBLISH" -eq 1 ]; then
  say "uploading to the ${TAG} draft release"
  if ! gh release view "$TAG" >/dev/null 2>&1; then
    gh release create "$TAG" --draft --title "signal ${VERSION}" \
      --notes "See the assets below. macOS builds are unsigned unless noted — first launch needs a right-click → Open, or \`xattr -dr com.apple.quarantine /Applications/signal.app\`."
  fi
  gh release upload "$TAG" "$DMG" "${DMG}.sha256" --clobber
  gh release view "$TAG" --json url --jq .url
fi

say "done"
echo "artifact: ${DMG}"
[ "$PUBLISH" -eq 1 ] || echo "not published — re-run with --publish to upload"
