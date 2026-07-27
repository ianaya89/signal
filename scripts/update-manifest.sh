#!/usr/bin/env bash
# Builds the updater manifest (latest.json) from the signatures attached to a
# release, and optionally uploads it there.
#
# The app polls
# https://github.com/ianaya89/signal/releases/latest/download/latest.json, so
# the manifest has to live on the release itself and point at that release's
# assets. Platforms whose .sig is missing are simply left out — a mac-only
# release still updates macs.
#
#   ./scripts/update-manifest.sh 0.2.0            # write dist-release/latest.json
#   ./scripts/update-manifest.sh 0.2.0 --publish  # ...and upload it
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:-}"
PUBLISH=0
[ "${2:-}" = "--publish" ] && PUBLISH=1
[ -n "$VERSION" ] || {
  echo "usage: ./scripts/update-manifest.sh <version> [--publish]" >&2
  exit 1
}

REPO="ianaya89/signal"
TAG="v${VERSION}"
DIST="dist-release"
OUT="${DIST}/latest.json"
BASE="https://github.com/${REPO}/releases/download/${TAG}"

say() { printf '\n\033[1;35m▶\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m✕\033[0m %s\n' "$*" >&2; exit 1; }

command -v gh >/dev/null || die "gh not found"
gh release view "$TAG" >/dev/null 2>&1 || die "no release ${TAG} — create it first"

mkdir -p "$DIST"
SIGS="$(mktemp -d)"
trap 'rm -rf "$SIGS"' EXIT HUP INT TERM

say "collecting signatures from ${TAG}"
gh release download "$TAG" --pattern '*.sig' --dir "$SIGS" --clobber 2>/dev/null || true
# A local run has the mac signature on disk before it is ever uploaded.
for local_sig in "${DIST}"/*.sig; do
  if [ -e "$local_sig" ]; then cp -f "$local_sig" "$SIGS/"; fi
done

# platform key -> asset carrying the update for it
PLATFORMS=(
  "darwin-aarch64:signal_${VERSION}_arm64.app.tar.gz"
  "darwin-x86_64:signal_${VERSION}_x86_64.app.tar.gz"
  "linux-x86_64:signal_${VERSION}_amd64.AppImage"
)

ENTRIES=""
for entry in "${PLATFORMS[@]}"; do
  key="${entry%%:*}"
  asset="${entry#*:}"
  sig_file="${SIGS}/${asset}.sig"
  if [ ! -s "$sig_file" ]; then
    echo "  ${key}: no ${asset}.sig — skipped"
    continue
  fi
  sig="$(tr -d '\n' < "$sig_file")"
  echo "  ${key}: ${asset}"
  [ -n "$ENTRIES" ] && ENTRIES="${ENTRIES},"
  ENTRIES="${ENTRIES}
    \"${key}\": {
      \"signature\": \"${sig}\",
      \"url\": \"${BASE}/${asset}\"
    }"
done

[ -n "$ENTRIES" ] || die "no signed artifacts found for ${TAG} — nothing to publish"

cat > "$OUT" <<EOF
{
  "version": "${VERSION}",
  "notes": "https://github.com/${REPO}/releases/tag/${TAG}",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "platforms": {${ENTRIES}
  }
}
EOF

# Malformed JSON here means every client silently stops updating.
python3 -m json.tool "$OUT" >/dev/null || die "generated ${OUT} is not valid JSON"
cat "$OUT"

if [ "$PUBLISH" -eq 1 ]; then
  say "uploading latest.json to ${TAG}"
  gh release upload "$TAG" "$OUT" --clobber
fi
