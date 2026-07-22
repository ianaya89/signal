# Design System

Signal has one visual theme, tuned for one purpose: a dense, dark, monospace, terminal-adjacent interface that reads more like LazyGit or Ghostty than a consumer media player. This document defines the color tokens, typography, spacing, component specs, motion budget, and voice rules that every screen is built from, plus a full ASCII mockup of the three-pane layout so the whole system can be seen in one place. Everything here targets React/TS/Tailwind/shadcn — tokens are plain CSS custom properties consumed through Tailwind's arbitrary-value syntax so there is exactly one source of truth per value.

## Design principles distilled

- **Density over whitespace.** Screen space is spent on information, not on breathing room. Row heights, paddings, and gaps are the minimum that keeps text legible and click/tap targets usable — not the minimum that looks "airy."
- **Information over decoration.** Every pixel either conveys a fact (codec, sample rate, rating, queue position) or supports scanning that fact (a border, a hint, an alignment). Nothing is drawn purely to look nice.
- **Instant over animated.** State changes appear immediately. Animation is reserved for the handful of cases in the Motion section where it prevents disorientation (focus moving, an overlay appearing) — never for delight on its own.
- **One theme done perfectly.** There is no light mode and no theme picker. All design effort goes into a single, carefully tuned dark palette instead of being spread thin across a matrix of variants that would each be worse for it.

## Color palette

GitHub-Dark-adjacent: muted, low-saturation surfaces with a single restrained cyan accent. All tokens are defined once in `:root` and consumed everywhere via `var(--token)` — no component hardcodes a hex value.

```css
:root {
  /* ---- surfaces ---- */
  --bg-base: #0d1117;      /* app chrome background: window frame, gaps between panes */
  --bg-surface: #161b22;   /* pane bodies: default background for table rows, list items, panels */
  --bg-raised: #1c2128;    /* elevated content above bg-surface: selected rows, palette overlay, kbd chips, popovers */

  /* ---- borders ---- */
  --border-subtle: #30363d; /* every structural 1px border: pane frames (unfocused), row dividers, input outlines at rest */
  --border-focus: #3fb9c9;  /* focused pane frame, focused input outline — equal to --accent by design, focus IS the accent's job */

  /* ---- text ---- */
  --text-primary: #e6edf3;  /* primary content: track titles, headings, values the user came to read */
  --text-secondary: #9198a1; /* secondary content: artist/album lines, field labels, palette descriptions */
  --text-muted: #6e7681;    /* tertiary: timestamps, placeholder text, disabled state, axis labels, log level TRACE/DEBUG */

  /* ---- accent ---- */
  --accent: #3fb9c9;      /* the one accent: active/playing state, focus rings, links, primary chart series */
  --accent-dim: #1f6b73;  /* low-emphasis accent: inactive glyph tint, subtle hover backgrounds mixed at low alpha */

  /* ---- semantic ---- */
  --ok: #3fb950;    /* success toasts, connected/healthy states */
  --warn: #d29922;  /* warning toasts, degraded states (e.g. device fallback) */
  --error: #f85149; /* error toasts, failed states, ERROR log lines */
  --info: #58a6ff;  /* informational toasts, INFO log lines, neutral notices */

  /* ---- audio-specific ---- */
  --bitperfect: #3fb950; /* bit-perfect output indicator (green — "this is correct") */
  --lossy: #d29922;      /* lossy codec badge (amber — "this is a compromise, not wrong") */
  --hires: #39c5cf;      /* hi-res badge (24-bit and/or >48kHz) — intentionally near --accent; hi-res audio and Signal's own brand cyan share the same "premium" cyan family */
}
```

Because Signal is dark-only, there is no `prefers-color-scheme: light` branch and no `data-theme="light"` override anywhere in the app — every component is written and tested against exactly this palette.

### Tailwind mapping

The CSS custom properties above are the source of truth; `tailwind.config.ts` maps each one to a semantic color name so components write `bg-surface`/`text-muted`/`border-focus` instead of arbitrary-value `var(...)` calls everywhere:

```ts
// tailwind.config.ts (excerpt)
colors: {
  base: "var(--bg-base)",
  surface: "var(--bg-surface)",
  raised: "var(--bg-raised)",
  border: {
    subtle: "var(--border-subtle)",
    focus: "var(--border-focus)",
  },
  fg: {
    primary: "var(--text-primary)",
    secondary: "var(--text-secondary)",
    muted: "var(--text-muted)",
  },
  accent: { DEFAULT: "var(--accent)", dim: "var(--accent-dim)" },
  ok: "var(--ok)", warn: "var(--warn)", error: "var(--error)", info: "var(--info)",
  bitperfect: "var(--bitperfect)", lossy: "var(--lossy)", hires: "var(--hires)",
}
```

Component sketches in this document use the raw `var(--token)` arbitrary-value form for clarity and copy-paste independence from the Tailwind config, but the actual codebase should prefer the mapped names (`bg-surface` over `bg-[var(--bg-surface)]`) once this config exists — one token still backs both.

## Typography

Monospace-first, everywhere. There is no proportional font in the MVP.

```css
--font-mono: ui-monospace, "JetBrains Mono", "SF Mono", "Cascadia Code", Menlo, Consolas, monospace;
```

Type scale (six sizes, no more):

| Token | Size | Use |
|---|---|---|
| `--text-2xs` | 11px | Axis labels, log timestamps, kbd chip glyphs |
| `--text-xs` | 12px | Status bar, badges, secondary table columns |
| `--text-sm` | 13px | Default body copy, most table cells |
| `--text-base` | 14px | Primary content: track titles, pane content default |
| `--text-lg` | 16px | Section headers, palette input |
| `--text-xl` | 20px | Rare emphasis only (e.g. a large "now playing" title in an expanded view) |

- **Tabular numbers everywhere.** Any numeric value that appears in a column, a timer, or next to another number gets `font-variant-numeric: tabular-nums;` — applied globally to table cells, the transport clock, bitrate/sample-rate badges, and stats views, so digits never cause horizontal jitter as they change.
- **Line height is 1.4** as the base for readable text (list rows, palette results). Compact-density table rows (see Spacing) use a tighter effective line height of 1.2 because the row height itself, not line-height, is doing the density work there.
- **Where a proportional font would be permitted:** nowhere in the MVP. The only plausible future case is long-form text that isn't Signal's own UI chrome — plugin-sourced artist bios or synced lyrics display — and even then it is deferred; monospace is used for that content today too, for consistency, until there's a concrete reason to special-case it.

## Spacing and density

4px base grid. Every margin, padding, and gap is a multiple of 4px.

| Token | Value |
|---|---|
| `--space-1` | 4px |
| `--space-2` | 8px |
| `--space-3` | 12px |
| `--space-4` | 16px |
| `--space-6` | 24px |
| `--space-8` | 32px |

- **Track table row height:** 28px default (`h-7` in Tailwind's 4px scale), 24px in compact density mode (`h-6`) — a user-facing density toggle, not two different designs.
- **Border radius:** 4px is the hard maximum, used only for the command palette overlay and toasts. Everything else — buttons, inputs, badges, kbd chips, pane containers — uses 2px, or no radius at all for full-bleed pane frames.
- **Borders are 1px only**, everywhere, always `--border-subtle` at rest. Nothing in Signal uses a 2px+ structural border; where extra emphasis is needed (the playing-track left bar, described below) that's a distinct decorative element, not a thicker border.
- **No shadows**, with one exception: the command palette overlay gets a shadow to visually lift it off the dimmed backdrop, since it's the one surface that floats above the rest of the UI rather than sitting flush in the pane grid.

## Component specs

### Track table row

Anatomy: index/artwork thumbnail, title, artist, album, duration, codec badge, rating. Four states.

| State | Treatment |
|---|---|
| Default | `--bg-surface` background, `--text-primary` title, `--text-secondary` artist/album |
| Hover | Background steps up to a hover tint between `--bg-surface` and `--bg-raised` |
| Selected | `--bg-raised` background (persists without hover, keyboard-navigable) |
| Playing | 2px `--accent` left bar + `--accent` title text color, on top of whatever selected/hover state also applies |

```html
<tr class="h-7 border-b border-[var(--border-subtle)]
           hover:bg-[var(--bg-raised)]/60
           data-[selected=true]:bg-[var(--bg-raised)]
           data-[playing=true]:border-l-2 data-[playing=true]:border-l-[var(--accent)]
           data-[playing=true]:[&_td.title]:text-[var(--accent)]">
```

### Pane container with title bar

Anatomy: a 24px title bar (pane name, optional count, contextual hint) over a scrollable body. Focused pane gets a full 1px `--border-focus` frame, the same visual language LazyGit uses to show which panel has keyboard focus.

```html
<div class="flex flex-col border border-[var(--border-subtle)]
            data-[focused=true]:border-[var(--border-focus)]
            transition-[border-color] duration-[120ms]">
  <div class="h-6 px-2 flex items-center justify-between
              text-xs text-[var(--text-secondary)]
              border-b border-[var(--border-subtle)]">
    <span>library</span>
    <span class="text-[var(--text-muted)]">1</span>
  </div>
  <div class="flex-1 overflow-y-auto">…</div>
</div>
```

### Codec/bit-depth/sample-rate badge

Bracketed, terminal-style chips — `[FLAC]` `[24/96]` — colored by what they signal, not by fixed per-codec color:

```html
<span class="inline-flex items-center h-4 px-1 text-[11px] leading-none
             font-mono border border-[var(--border-subtle)] rounded-sm
             bg-[var(--bg-surface)] text-[var(--hires)]">24/96</span>

<span class="inline-flex items-center h-4 px-1 text-[11px] leading-none
             font-mono border border-[var(--border-subtle)] rounded-sm
             bg-[var(--bg-surface)] text-[var(--lossy)]">MP3</span>
```

Color rule: `--hires` when the track is 24-bit and/or above 48kHz, `--bitperfect` when the current output path is confirmed bit-perfect for that track, `--lossy` for any lossy codec (MP3/AAC/Ogg), `--text-secondary` (no special color) for standard lossless (16-bit/44.1–48kHz FLAC/ALAC) — the neutral case doesn't need a warning or a celebration color.

### Keyboard hint (`kbd`)

```html
<kbd class="inline-flex items-center justify-center h-4 min-w-4 px-1
            text-[10px] leading-none font-mono
            text-[var(--text-secondary)] bg-[var(--bg-raised)]
            border border-[var(--border-subtle)] rounded-sm">j</kbd>
```

Multi-key hints (`gg`, `r 1-5`) render as two adjacent `kbd` elements with a 2px gap, never as one chip with a literal space inside it — this keeps each physical keypress visually distinct.

### Status bar segments

Bottom bar, three zones: mode/hints on the left, transport in the center, device/volume on the right. See the full mockup below for realistic content.

```html
<div class="h-6 flex items-center gap-3 px-2
            text-xs text-[var(--text-secondary)]
            bg-[var(--bg-surface)] border-t border-[var(--border-subtle)]">
```

### Command palette overlay

Centered, fixed 560px width, capped-height result list.

```html
<div class="fixed inset-0 bg-black/60 flex items-start justify-center pt-24">
  <div class="w-[560px] bg-[var(--bg-raised)] border border-[var(--border-subtle)]
              rounded shadow-lg">
    <input class="w-full h-9 px-3 bg-transparent text-sm
                  text-[var(--text-primary)] border-b border-[var(--border-subtle)]
                  outline-none placeholder:text-[var(--text-muted)]"
           placeholder="type a command…" />
    <ul class="max-h-80 overflow-y-auto py-1">
      <li class="flex items-center justify-between px-3 h-7 text-sm
                 text-[var(--text-primary)]
                 data-[active=true]:bg-[var(--bg-surface)]">
        <span>queue: clear</span>
        <kbd class="text-[var(--text-muted)]">c</kbd>
      </li>
    </ul>
  </div>
</div>
```

### Toast

```html
<div class="flex items-center gap-2 h-8 px-3
            bg-[var(--bg-raised)] border border-[var(--border-subtle)]
            border-l-2 border-l-[var(--error)] rounded
            text-xs text-[var(--text-primary)]">
  playback failed: unsupported codec — see logs (L)
</div>
```

The left border color is the only thing that changes per level: `--error`, `--warn`, `--ok`, `--info`.

### Progress/seek bar

2px line at rest, no thumb until hover.

```html
<div class="relative h-[2px] bg-[var(--border-subtle)] group cursor-pointer">
  <div class="absolute inset-y-0 left-0 bg-[var(--accent)]" style="width: 32%"></div>
  <div class="absolute top-1/2 -translate-y-1/2 h-2 w-2 rounded-full
              bg-[var(--accent)] opacity-0 group-hover:opacity-100"
       style="left: 32%"></div>
</div>
```

### Volume indicator

A short (48px) version of the same seek-bar visual language, plus a tabular-nums percentage — never a slider widget with a visible track/thumb at rest, to match the seek bar's restraint:

```html
<div class="flex items-center gap-2 text-xs text-[var(--text-secondary)]">
  <div class="relative w-12 h-[2px] bg-[var(--border-subtle)]">
    <div class="absolute inset-y-0 left-0 bg-[var(--accent)]" style="width: 72%"></div>
  </div>
  <span class="tabular-nums">72%</span>
</div>
```

### Log line coloring by level

```html
<div class="font-mono text-xs leading-relaxed flex gap-2">
  <span class="text-[var(--text-muted)] tabular-nums">12:03:41</span>
  <span class="text-[var(--error)]">ERROR</span>
  <span class="text-[var(--text-secondary)]">signal_scanner</span>
  <span class="text-[var(--text-primary)]">failed to probe file: unsupported container</span>
</div>
```

Level-to-color mapping: `TRACE`/`DEBUG` → `--text-muted`, `INFO` → `--info`, `WARN` → `--warn`, `ERROR` → `--error`. Only the level token itself is colored; the message stays `--text-primary` so colored noise doesn't fight legibility on a busy log stream.

## Motion

Signal's default animation duration is **0ms**. Two exceptions exist, and nothing else:

| Transition | Duration | Easing |
|---|---|---|
| Command palette open (opacity) | 80ms | ease-out |
| Pane focus border-color change | 120ms | ease-out |

`ease-out` is the single easing curve used anywhere in the app — nothing gets a spring, a bounce, or a custom cubic-bezier. `prefers-reduced-motion: reduce` disables both of these listed transitions entirely (the palette appears instantly, the focus border changes color instantly) rather than just shortening them — reduced motion means no motion, not less motion.

## Iconography

Minimal by default: text glyphs are preferred over icon components wherever a glyph is unambiguous — `▶` `⏸` `⏭` `⏮` `♥` `⚡` (bit-perfect) cover most transport and status needs and render for free in the monospace font with zero extra asset weight. Lucide icons are used only where no glyph reads clearly at a glance:

| Case | Icon | Why not a glyph |
|---|---|---|
| Settings | `lucide:settings` | No universally-read gear glyph exists in standard fonts |
| Folder / reveal-in-file-manager | `lucide:folder-open` | Emoji folder glyphs render inconsistently across platforms |
| Drag handle (queue reorder, fallback for non-keyboard use) | `lucide:grip-vertical` | No monospace glyph reads as "draggable" |
| Playlist | `lucide:list-music` | Distinguishes from the plain queue/list glyph used elsewhere |
| Device/output picker | `lucide:speaker` | Needed to distinguish output device from volume level at a glance |

All are rendered at 14px, stroke-only (never filled), colored via `currentColor` so they inherit whichever text token they're placed against rather than carrying their own color.

## Data-viz style (uPlot)

- **Single accent color** (`--accent`) for the primary series in every chart — listening-activity over time, per-format breakdown, whatever the stat is. Signal doesn't build multi-series categorical palettes; where more than one series is unavoidable, the secondary series uses `--text-secondary` rather than introducing a second hue.
- **Gridlines** are `--border-subtle`, always thin and always secondary to the data.
- **Heatmaps** (e.g. a GitHub-style listening-activity calendar) use an alpha ramp of `--accent` over `--bg-base` — darkest cell is `--bg-base` itself (no activity), brightest is `--accent` at full opacity, with no separate color scale.
- **Axis text** is `--text-muted` at 11px (`--text-2xs`) — present, legible, and deliberately quiet relative to the data itself.
- **Tooltips** are styled like the toast component: `--bg-raised` background, `--border-subtle` 1px border, monospace tabular numbers for every value shown.

## Voice and copy

- Lowercase labels are fine for UI chrome (`queue`, `now playing`, `search`) — this is a terminal-adjacent tool, not a magazine layout, and lowercase reduces visual noise in a dense UI. Proper nouns (artist names, album titles, actual file paths) are always shown verbatim, case untouched.
- Technical values are shown exactly as they are, never translated into consumer-friendly names: `44.1kHz`, not "CD Quality"; `16-bit`, not "Standard Quality"; `FLAC`, not "Lossless Audio Format." The audience for this app already knows what these mean, and softening them loses precision for no gain.
- Empty states are a single line: what's missing, plus the keyboard shortcut that fixes it. Example: `no tracks in queue — press a on a track to stage it`. Never a multi-paragraph explanation, never an illustration.
- Error copy is exactly two things: what failed, and where to look next. Example: `playback failed: unsupported codec — see logs (L) for details`. It never apologizes, and it never hides the technical reason behind a vague "something went wrong."

## Full layout mockup

```
┌─ library ───────────────┬─ cerati · bocanada (1999) ─────────────────focused─┬─ inspector ──────────────┐
│ artists  albums  genres  │ #  title              artist            time  codec   rtg          │ FLAC  24-bit / 96.0kHz    │
│                          │ 1  balería             Gustavo Cerati    4:12  [FLAC]  ★★★★☆         │ replaygain  -6.2 dB (trk) │
│ ▸ Charly García          │▶2  bocanada            Gustavo Cerati    5:33  [FLAC]  ★★★★★         │ peak        -0.4 dBFS     │
│ ▾ Gustavo Cerati         │ 3  verbo carne         Gustavo Cerati    4:47  [FLAC]  ★★★☆☆         │ dr           11           │
│    bocanada (1999)       │ 4  puente              Gustavo Cerati    4:05  [FLAC]  ★★★★☆         │ device      Scarlett 2i2  │
│    ahí vamos (2006)      │ 5  engaña               Gustavo Cerati    3:58  [FLAC]  ★★★☆☆         │ [BIT-PERFECT]             │
│ ▸ Fito Páez               │ 6  (bocanada) balería  Gustavo Cerati    6:21  [FLAC]  ★★★★☆         ├─ queue ───────────────────┤
│ ▸ Soda Stereo            │                                                                       │  2  bocanada         4:33 │
│                          │                                                                       │  3  verbo carne      0:00 │
│                          │                                                                       │  4  puente           0:00 │
│                          │                                                                       │ 12  frágil (soda)    0:00 │
└──────────────────────────┴───────────────────────────────────────────────────────────────────────┴────────────────────────────┘
:                                                                                                                                 ▏
 NORMAL  j/k move · Enter play · a stage · A play next · f fav · r rate · / search · : cmd · ? help
 ▶ bocanada — Gustavo Cerati    [────────●──────────────────────────]   1:47 / 5:33   FLAC 24/96   ♪ 72%   ⚡ bit-perfect
```

Reading the mockup against the specs above: the Center pane carries `focused` styling (a `--border-focus` frame the ASCII renders as its own border row), row 2 is `playing` (accent-colored row, marked here with `▶`), the codec badges are bracketed `[FLAC]` chips, the Inspector's `[BIT-PERFECT]` line uses the `--bitperfect` token, the command line sits collapsed at the very bottom of the pane stack ready for `:`, and the status bar shows the exact contextual-hint pattern described in `09-keyboard.md` — pane-specific hints on the left, transport with a 2px seek line in the center, device/volume on the right.
