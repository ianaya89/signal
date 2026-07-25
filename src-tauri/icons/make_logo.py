"""Signal app icon: heart built from vertical audio bars.

Dark-theme palette, diagonal indigo gradient bg, rounded (squircle-ish)
icon corners, per-bar rounded caps, deliberate non-uniform bar heights.
Pure stdlib (zlib PNG writer)."""

import struct, zlib, sys, math, random

SIZE = 1024
# macOS icon grid: the squircle body fills ~82.4% of the canvas, the rest
# is transparent margin — edge-to-edge icons look oversized in the Dock.
MARGIN = int(SIZE * 0.088)
BODY = SIZE - 2 * MARGIN
CORNER_R = int(BODY * 0.225)

# dark blue-violet theme
BG_TOP = (0x1B, 0x1B, 0x2E)   # lighter indigo, top-left
BG_BOT = (0x0E, 0x0E, 0x18)   # near-black, bottom-right
BAR_A = (0x82, 0x86, 0xF5)    # periwinkle accent
BAR_B = (0xBB, 0x9A, 0xF7)    # violet (hires)

random.seed(41)

def heart_ys(x):
    """Column extents of the classic implicit heart, x in [-1.25, 1.25]."""
    lo, hi = None, None
    steps = 700
    for i in range(steps):
        y = -1.3 + 2.8 * i / steps
        v = (x * x + y * y - 1) ** 3 - x * x * y ** 3
        if v <= 0:
            if lo is None:
                lo = y
            hi = y
    return lo, hi

N_BARS = 11
bar_w = BODY * 0.052
gap = BODY * 0.026
total = N_BARS * bar_w + (N_BARS - 1) * gap
x0 = (SIZE - total) / 2
scale = BODY * 0.30
cy = MARGIN + BODY * 0.54

bars = []
for i in range(N_BARS):
    cx_px = x0 + i * (bar_w + gap) + bar_w / 2
    hx = (cx_px - SIZE / 2) / scale
    lo, hi = heart_ys(hx)
    if lo is None:
        continue
    jitter_top = random.uniform(-0.09, 0.06)
    jitter_bot = random.uniform(-0.05, 0.08)
    y_top = cy - (hi + jitter_top) * scale
    y_bot = cy - (lo + jitter_bot) * scale
    if y_bot - y_top < bar_w:
        y_bot = y_top + bar_w
    t = i / (N_BARS - 1)
    col = tuple(int(BAR_A[c] + (BAR_B[c] - BAR_A[c]) * t) for c in range(3))
    bars.append((cx_px, y_top, y_bot, col))

half_w = bar_w / 2

def bar_alpha(px, py):
    """Coverage + color for bar pixels (rounded capsule ends)."""
    for cx_px, y_top, y_bot, col in bars:
        dx = abs(px - cx_px)
        if dx > half_w + 1:
            continue
        if y_top + half_w <= py <= y_bot - half_w:
            d = dx
        else:
            cyy = y_top + half_w if py < y_top + half_w else y_bot - half_w
            d = math.hypot(px - cx_px, py - cyy)
        if d <= half_w - 1:
            return col, 1.0
        if d <= half_w + 1:
            return col, (half_w + 1 - d) / 2
    return None, 0.0

def corner_alpha(px, py):
    """Rounded-rect mask (inset body) with 2px AA edge."""
    x0b, x1b = MARGIN, SIZE - 1 - MARGIN
    if px < x0b or px > x1b or py < x0b or py > x1b:
        return 0.0
    r = CORNER_R
    x = min(px - x0b, x1b - px)
    y = min(py - x0b, x1b - py)
    if x >= r or y >= r:
        return 1.0
    d = math.hypot(r - x, r - y)
    if d <= r - 1:
        return 1.0
    if d >= r + 1:
        return 0.0
    return (r + 1 - d) / 2

rows = []
for py in range(SIZE):
    row = bytearray()
    for px in range(SIZE):
        t = (max(px - MARGIN, 0) + max(py - MARGIN, 0)) / (2 * BODY)
        bg = tuple(int(BG_TOP[c] + (BG_BOT[c] - BG_TOP[c]) * t) for c in range(3))
        col, a = bar_alpha(px, py)
        if col and a > 0:
            pixel = tuple(int(bg[c] * (1 - a) + col[c] * a) for c in range(3))
        else:
            pixel = bg
        mask = corner_alpha(px, py)
        row += bytes(pixel) + bytes([int(255 * mask)])
    rows.append(bytes(row))

raw = b"".join(b"\x00" + r for r in rows)

def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data))

png = (b"\x89PNG\r\n\x1a\n"
       + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
       + chunk(b"IDAT", zlib.compress(raw, 9))
       + chunk(b"IEND", b""))

with open(sys.argv[1], "wb") as f:
    f.write(png)
print("wrote", sys.argv[1], len(png), "bytes")
