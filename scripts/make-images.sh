#!/usr/bin/env bash
# Renders the README banner and the app screenshot from docs/index.html itself
# (via its ?shot= mode), so the images can never drift from the live design
# system. Requires Google Chrome and Python with Pillow.
#
#   ./scripts/make-images.sh
set -euo pipefail
cd "$(dirname "$0")/.."

CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
[ -x "$CHROME" ] || { echo "error: Chrome not found at $CHROME (set CHROME=...)"; exit 1; }

OUT=docs/assets
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"

shoot() {
  # shoot <shot-name> <window-w> <window-h>
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars \
    --force-device-scale-factor=2 --virtual-time-budget=4000 \
    --window-size="$2,$3" --screenshot="$TMP/$1.png" \
    "file://$PWD/docs/index.html?shot=$1" >/dev/null 2>&1
}

# Cover: the ?shot=cover body is a fixed 1280x640 poster frame — keep it whole.
shoot cover 1280 520
cp "$TMP/cover.png" "$OUT/cover.png"

# App: window is deliberately oversized so nothing clips, then trimmed back to
# the mockup's bounding box.
shoot app 1080 1000
python3 - "$TMP/app.png" "$OUT/app.png" <<'PY'
import sys
from PIL import Image

src, dst = sys.argv[1], sys.argv[2]
im = Image.open(src).convert("RGB")
px = im.load()
w, h = im.size
bg = px[w - 2, h - 2]

# Tolerance keeps the faint page grid from defeating the trim.
def differs(x, y):
    p = px[x, y]
    return max(abs(p[i] - bg[i]) for i in range(3)) > 8

left, right, top, bottom = w, -1, h, -1
for y in range(0, h, 2):
    for x in range(0, w, 2):
        if differs(x, y):
            if x < left: left = x
            if x > right: right = x
            if y < top: top = y
            if y > bottom: bottom = y

if right < 0:
    im.save(dst)
    raise SystemExit(0)

pad = 20
box = (max(0, left - pad), max(0, top - pad), min(w, right + pad), min(h, bottom + pad))
im.crop(box).save(dst)
print(f"{dst}: {box[2] - box[0]}x{box[3] - box[1]}")
PY

ls -lh "$OUT/cover.png" "$OUT/app.png"
