# Design System

Signal has two themes — dark and light — tuned for one purpose: a dense, monospace, terminal-adjacent interface that reads more like LazyGit or Ghostty than a consumer media player. This document defines the color tokens, typography, spacing, component specs, motion budget, and voice rules that every screen is built from, plus a full ASCII mockup of the three-pane layout so the whole system can be seen in one place. Everything here targets React/TS/Tailwind 4/shadcn — tokens are plain CSS custom properties, bridged into Tailwind's utility classes through a single `@theme inline` block, so there is exactly one source of truth per value.

## Design principles distilled

- **Density over whitespace.** Screen space is spent on information, not on breathing room. Row heights, paddings, and gaps are the minimum that keeps text legible and click/tap targets usable — not the minimum that looks "airy."
- **Information over decoration.** Every pixel either conveys a fact (codec, sample rate, rating, queue position) or supports scanning that fact (a border, a hint, an alignment). Nothing is drawn purely to look nice.
- **Instant over animated.** State changes appear immediately. Animation is reserved for the handful of cases in the Motion section where it prevents disorientation or communicates a live process — never for delight on its own.
- **Two themes, one grammar.** Dark is the primary theme, but light is a full peer, not a filter over dark: its own hand-tuned "manila paper" palette sharing the same token names, the same weights, the same density. Every component in this document is expected to hold up unchanged in both.

## Color palette

Two palettes, switched by stamping `data-theme` on the root element. Dark (`:root`) is a near-black indigo — deep blue-violet surfaces with a periwinkle accent. Light (`:root[data-theme="light"]`) is a warm "manila paper" theme — ink text on paper surfaces with a sealing-wax accent. All tokens are defined once for dark in `:root` and re-declared for light in `:root[data-theme="light"]`, consumed everywhere via `var(--token)` — no component hardcodes a hex value, and no component branches on which theme is active.

```css
:root {
  /* ---- surfaces: near-black indigo ---- */
  --bg-base: #12121c;      /* window chrome background: window frame, gaps between panes */
  --bg-surface: #181826;   /* pane bodies: default background for table rows, list items, panels */
  --bg-raised: #232338;    /* elevated content above bg-surface: selected rows, popovers, kbd chips */

  /* ---- borders ---- */
  --border-subtle: #2e2e4a; /* every structural 1px border: pane frames (unfocused), row dividers, input outlines at rest */
  --border-focus: #8286f5;  /* focused pane frame, focused input outline — equal to --accent by design, focus IS the accent's job */

  /* ---- text ---- */
  --text-primary: #d8daf0;   /* primary content: track titles, headings, values the user came to read */
  --text-secondary: #a5a8cc; /* secondary content: artist/album lines, field labels */
  --text-muted: #63678f;     /* tertiary: timestamps, placeholders, hints */

  /* ---- accent: periwinkle blue-violet ---- */
  --accent: #8286f5;      /* the one accent: playing state, focus, links, primary chart series */
  --accent-dim: #4c4f96;  /* low-emphasis accent: inactive glyph tint, subtle hover backgrounds */

  /* ---- semantic ---- */
  --ok: #73daca;    /* teal — success toasts, connected/healthy states */
  --warn: #e0af68;  /* amber — warning toasts, degraded states */
  --error: #f7768e; /* rose — error toasts, failed states, ERROR log lines */
  --info: #7aa2f7;  /* blue — informational toasts, INFO log lines */

  /* ---- audio-specific ---- */
  --bitperfect: #73daca; /* teal — "this is correct" */
  --lossy: #e0af68;      /* amber — "this is a compromise, not wrong" */
  --hires: #bb9af7;      /* violet — hi-res badge (24-bit and/or >48kHz), the premium tier */
}
```

```css
:root[data-theme="light"] {
  /* ---- surfaces: manila paper ---- */
  --bg-base: #e8dec7;
  --bg-surface: #f2ead8;
  --bg-raised: #e0d3b4;

  /* ---- borders ---- */
  --border-subtle: #c9b995;
  --border-focus: #bf5b3f;

  /* ---- text: warm ink ---- */
  --text-primary: #3d3427;
  --text-secondary: #5f5340;
  --text-muted: #8f8163;

  /* ---- accent: sealing wax ---- */
  --accent: #bf5b3f;
  --accent-dim: #d9a08c;

  /* ---- semantic: stamp-pad inks ---- */
  --ok: #4a7c59;
  --warn: #a97b23;
  --error: #b3402e;
  --info: #5a6fae;

  /* ---- audio-specific ---- */
  --bitperfect: #4a7c59;
  --lossy: #a97b23;
  --hires: #7c5eb0;
}
```

Every component in the rest of this document is written and reviewed against both palettes; nothing in Signal assumes dark is the only option, even where it's the default.

### Tailwind mapping

The CSS custom properties above are the source of truth. There is no `tailwind.config.ts` — this is Tailwind 4, and the bridge is an `@theme inline` block inside `src/styles.css` itself, which is what makes `bg-surface`, `text-muted`, and `border-focus` exist as utility classes at all:

```css
@theme inline {
  --color-base: var(--bg-base);
  --color-surface: var(--bg-surface);
  --color-raised: var(--bg-raised);
  --color-subtle: var(--border-subtle);
  --color-focus: var(--border-focus);
  --color-primary: var(--text-primary);
  --color-secondary: var(--text-secondary);
  --color-muted: var(--text-muted);
  --color-accent: var(--accent);
  --color-accent-dim: var(--accent-dim);
  --color-ok: var(--ok);
  --color-warn: var(--warn);
  --color-error: var(--error);
  --color-info: var(--info);
  --color-bitperfect: var(--bitperfect);
  --color-lossy: var(--lossy);
  --color-hires: var(--hires);
  --font-mono: var(--font-mono);
  --radius-pane: var(--radius);
  --radius-row: var(--radius-sm);
}
```

Component sketches in this document use the mapped Tailwind names (`bg-surface`, `border-focus`) rather than raw `bg-[var(--bg-surface)]` arbitrary values, since the mapping exists precisely so components don't have to spell out `var(...)` everywhere.

Two gotchas worth knowing before touching this file:

- **Tailwind extracts class names by scanning source text, so it cannot see through a template literal.** Building a class string from interpolated fragments — `` `bg-[color:${ACCENT}]` `` — compiles to no CSS at all, silently, and the build still succeeds. Shared class constants (see Control vocabulary, below) are written out in full, every time, for exactly this reason.
- **A space inside an arbitrary value ends the class.** A `var()` fallback must be written `var(--section,var(--accent))` with no space after the comma — `var(--section, var(--accent))` reads as two tokens to the scanner and silently breaks.

### Channel hues

Settings and the system pane are both split into channels — library, playback, appearance, scrobbling, server, remote, about, stats, doctor, logs — and each owns a hue, so color carries wayfinding rather than decoration: you learn "amber is the server pane" the same way you'd learn a rack strip's channel colors. Ten tokens, one per channel, restated for each theme:

```css
/* dark */
--sec-library: #7aa2f7;    --sec-playback: #73daca;   --sec-appearance: #bb9af7;
--sec-scrobbling: #8286f5; --sec-server: #e0af68;     --sec-remote: #9ece6a;
--sec-about: #7f83ad;      --sec-stats: #bb9af7;      --sec-doctor: #e0af68;
--sec-logs: #7f83ad;

/* light — the same hues restated in ink rather than neon */
--sec-library: #5a6fae;    --sec-playback: #2f7d6a;   --sec-appearance: #7c5eb0;
--sec-scrobbling: #bf5b3f; --sec-server: #a97b23;     --sec-remote: #4a7c59;
--sec-about: #8f8163;      --sec-stats: #7c5eb0;      --sec-doctor: #a97b23;
--sec-logs: #8f8163;
```

The active channel publishes its hue as `--section` on the pane root (see `TabbedPane`, below), so anything rendered inside — focus rings, section ticks, hover states — picks it up without being told which pane it's in. The sidebar tints its active row with the hue of the pane that row opens, and the discover shelves each take one too.

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
- **Border radius is zero.** Both `--radius` and `--radius-sm` are `0px` — square corners everywhere, described in the stylesheet itself as "TUI, not toy." There is no 2px/4px radius scale to reach for; a `rounded-*` utility applied to a Signal component doesn't earn its keep, since the tokens it would round through (`--radius-pane`, `--radius-row`) resolve to nothing.
- **Borders are 1px only**, everywhere, always `--border-subtle` at rest. Nothing in Signal uses a 2px+ structural border; where extra emphasis is needed (the playing-track left bar, described below) that's a distinct decorative element, not a thicker border.
- **No shadows**, with one exception: the command palette overlay gets a shadow to visually lift it off the dimmed backdrop, since it's the one surface that floats above the rest of the UI rather than sitting flush in the pane grid.

## Component specs

### Control vocabulary (`src/components/ui/controls.ts`)

Three button weights and two input sizes, defined once because these class strings had been hand-copied — byte-identical in some places, drifted by a padding step in others — across settings, doctor, playlists and remote:

```ts
// src/components/ui/controls.ts (excerpt)

// The default weight. Most buttons are this — if everything is emphasised,
// nothing is. Secondary text on bg-raised; hover resolves to --section where
// the control sits inside a channel pane, and the global accent otherwise.
export const BTN =
  "shrink-0 border border-subtle bg-raised px-2 py-0.5 text-[11px] text-secondary " +
  "transition-colors hover:border-[color:var(--section,var(--border-focus))] " +
  "hover:text-[color:var(--section,var(--accent))]";

// The one action a view exists for: submit the form, start the server, play
// the album. Filled rather than outlined — in a flat, square-cornered UI,
// weight is the only hierarchy available, since there are no shadows or radii
// to lean on. At most one per context.
export const BTN_PRIMARY =
  "shrink-0 border border-[color:var(--accent-fill)] bg-[color:var(--accent-fill)] " +
  "px-2 py-0.5 text-[11px] font-semibold text-[color:var(--on-accent)] " +
  "transition-opacity hover:opacity-85 disabled:cursor-not-allowed disabled:opacity-40";

// Destructive actions carry the error color at rest, not only on hover.
export const BTN_DANGER =
  "shrink-0 border border-subtle bg-raised px-2 py-0.5 text-[11px] text-error/80 " +
  "transition-colors hover:border-error hover:text-error";

export const INPUT =
  "border border-subtle bg-base/60 px-2 py-0.5 text-[11px] text-primary outline-none " +
  "focus:border-[color:var(--section,var(--border-focus))]";

export const INPUT_LG = /* same as INPUT, at the larger step used by dialogs/editors */
  "border border-subtle bg-base/60 px-2 py-1 text-[12px] text-primary outline-none " +
  "focus:border-[color:var(--section,var(--border-focus))]";
```

`BTN`, `BTN_DANGER`, `INPUT`, and `INPUT_LG` all resolve their emphasis color to `--section` first, so a control looks native to whichever channel pane it lands in without being told which one that is.

`BTN_PRIMARY` deliberately breaks that pattern and does **not** pick up `--section`. Two reasons that agree: a primary tinted like its pane would blend into the pane instead of standing out from it, and half the channel hues can't carry a label at 4.5:1 on the manila theme (amber measures 3.7:1). Its fill and label are their own tokens instead, `--accent-fill` and `--on-accent`:

```css
/* dark */
--accent-fill: #8286f5;
--on-accent: #12121c;   /* → 5.9:1 */

/* light */
--accent-fill: #b5512f; /* a deeper wax than --accent, on purpose */
--on-accent: #fffdf8;   /* → 4.9:1 */
```

The light fill has to be deeper than `--accent`: `--accent: #bf5b3f` under a near-white label measures only 3.3:1, under AA.

### Known accessibility gap

Worth recording honestly rather than glossing over: secondary button hover text (`BTN`'s hover state) on the manila theme measures about 2.6:1 against `--bg-raised` — it was around 3.0:1 before the channel hues shifted the hover color, and both figures are under AA. The dark theme's equivalent hover improves to 7.7:1 from 4.9:1 over the same change. This is a light-palette weakness, not a regression introduced by any one change, and it is currently unfixed.

### Track table row

Anatomy: index/artwork thumbnail, title, artist, album, duration, codec badge, rating. Four states.

| State | Treatment |
|---|---|
| Default | `--bg-surface` background, `--text-primary` title, `--text-secondary` artist/album |
| Hover | Background steps up to a hover tint between `--bg-surface` and `--bg-raised` |
| Selected | `--bg-raised` background (persists without hover, keyboard-navigable) |
| Playing | 2px `--accent` left bar + `--accent` title text color, on top of whatever selected/hover state also applies |

```html
<tr class="h-7 border-b border-subtle
           hover:bg-raised/60
           data-[selected=true]:bg-raised
           data-[playing=true]:border-l-2 data-[playing=true]:border-l-accent
           data-[playing=true]:[&_td.title]:text-accent">
```

### Pane container with title bar

Anatomy: a 24px title bar (pane name, optional count, contextual hint) over a scrollable body. Focused pane gets a full 1px `--border-focus` frame, the same visual language LazyGit uses to show which panel has keyboard focus.

```html
<div class="flex flex-col border border-subtle
            data-[focused=true]:border-focus
            transition-[border-color] duration-[120ms]">
  <div class="h-6 px-2 flex items-center justify-between
              text-xs text-secondary
              border-b border-subtle">
    <span>library</span>
    <span class="text-muted">1</span>
  </div>
  <div class="flex-1 overflow-y-auto">…</div>
</div>
```

The title bar also carries the row count of whatever the view is showing (`useMainTitle("albums", albums.length)` → `[ albums ] 59`), dim and outside the brackets. It is status, not a label, so it never joins the `·` chain a detail title uses (`album · Kid A`).

### Keyboard focus

One global rule in `styles.css`:

```css
:focus-visible { outline: 1px solid var(--border-focus); outline-offset: 1px; }
```

Controls style `hover:` by hand and none of them styled focus, so tabbing moved an invisible cursor. `:focus-visible` (not `:focus`) keeps the ring off pointer clicks. Inputs still override it with their `focus:border-focus` treatment — a border reads better on a field than a ring.

### Pane title bar actions (`src/components/ui/PaneActions.tsx`)

**Rule: a view gets no toolbar of its own.** Sorts, filters and the one or two actions a view exists for render on the right of the main pane's title bar, opposite `[ albums ]`. Views publish them with `<PaneActions>`, which portals into a slot the pane header exposes (`uiStore.mainHeaderSlot`).

Why it is a rule and not a preference:

- A second strip under the title bar repeated the bar's job one line lower and cost a row of vertical space in every list.
- A sticky `GroupHeader` cannot rise past its scroll container's padding box, so a padded list under a toolbar showed a band of scrolled content above the stuck header. Removing the strip removed the seam.
- One place to look for "what can I do here", per view, at a fixed screen position.

```tsx
<PaneActions>
  <PaneSort value={sort} options={SORTS} onChange={setSort} />
  <PaneActionsDivider />
  <PaneAction tone="primary" onClick={playAll}>▶ play all</PaneAction>
  <PaneAction onClick={queueAll}>+ queue all</PaneAction>
</PaneActions>
```

`PaneAction` comes in two weights, and the difference is fill, not size: `tone="primary"` is tinted at rest (`--accent-dim` wash, accent label) for the action that starts playback; everything else is an outline on `--bg-raised`. At most one primary per view. Buttons are separated by `gap-1` — the header lays them out, so a view never hand-spaces them.

Content headers (album art + title on a detail view) stay in the body; they are the record, not the controls. Breadcrumbs stay in the body too — they are where you are, not what you can do.

### Codec/bit-depth/sample-rate badge

Bracketed, terminal-style chips — `[FLAC]` `[24/96]` — colored by what they signal, not by fixed per-codec color:

```html
<span class="inline-flex items-center h-4 px-1 text-[11px] leading-none
             font-mono border border-subtle
             bg-surface text-hires">24/96</span>

<span class="inline-flex items-center h-4 px-1 text-[11px] leading-none
             font-mono border border-subtle
             bg-surface text-lossy">MP3</span>
```

Color rule: `--hires` when the track is 24-bit and/or above 48kHz, `--bitperfect` when the current output path is confirmed bit-perfect for that track, `--lossy` for any lossy codec (MP3/AAC/Ogg), `--text-secondary` (no special color) for standard lossless (16-bit/44.1–48kHz FLAC/ALAC) — the neutral case doesn't need a warning or a celebration color.

### Keyboard hint (`kbd`)

```html
<kbd class="inline-flex items-center justify-center h-4 min-w-4 px-1
            text-[10px] leading-none font-mono
            text-secondary bg-raised
            border border-subtle">j</kbd>
```

Multi-key hints (`gg`, `r 1-5`) render as two adjacent `kbd` elements with a 2px gap, never as one chip with a literal space inside it — this keeps each physical keypress visually distinct.

### Status bar segments

Bottom bar, three zones: mode/hints on the left, transport in the center, device/volume on the right. See the full mockup below for realistic content.

```html
<div class="h-6 flex items-center gap-3 px-2
            text-xs text-secondary
            bg-surface border-t border-subtle">
```

### Command palette overlay

Centered, fixed 560px width, capped-height result list.

```html
<div class="fixed inset-0 bg-black/60 flex items-start justify-center pt-24">
  <div class="w-[560px] bg-raised border border-subtle shadow-lg">
    <input class="w-full h-9 px-3 bg-transparent text-sm
                  text-primary border-b border-subtle
                  outline-none placeholder:text-muted"
           placeholder="type a command…" />
    <ul class="max-h-80 overflow-y-auto py-1">
      <li class="flex items-center justify-between px-3 h-7 text-sm
                 text-primary
                 data-[active=true]:bg-surface">
        <span>queue: clear</span>
        <kbd class="text-muted">c</kbd>
      </li>
    </ul>
  </div>
</div>
```

### Toast

```html
<div class="flex items-center gap-2 h-8 px-3
            bg-raised border border-subtle
            border-l-2 border-l-error
            text-xs text-primary">
  playback failed: unsupported codec — see logs (L)
</div>
```

The left border color is the only thing that changes per level: `--error`, `--warn`, `--ok`, `--info`.

### Progress/seek bar

2px line at rest, no thumb until hover. The thumb is a square block, not a circle — corners are square everywhere in Signal, including on hover affordances.

```html
<div class="relative h-[2px] bg-subtle group cursor-pointer">
  <div class="absolute inset-y-0 left-0 bg-accent" style="width: 32%"></div>
  <div class="absolute top-1/2 -translate-y-1/2 h-2 w-2
              bg-accent opacity-0 group-hover:opacity-100"
       style="left: 32%"></div>
</div>
```

### Volume indicator

A short (48px) version of the same seek-bar visual language, plus a tabular-nums percentage — never a slider widget with a visible track/thumb at rest, to match the seek bar's restraint:

```html
<div class="flex items-center gap-2 text-xs text-secondary">
  <div class="relative w-12 h-[2px] bg-subtle">
    <div class="absolute inset-y-0 left-0 bg-accent" style="width: 72%"></div>
  </div>
  <span class="tabular-nums">72%</span>
</div>
```

### Log line coloring by level

```html
<div class="font-mono text-xs leading-relaxed flex gap-2">
  <span class="text-muted tabular-nums">12:03:41</span>
  <span class="text-error">ERROR</span>
  <span class="text-secondary">signal_scanner</span>
  <span class="text-primary">failed to probe file: unsupported container</span>
</div>
```

Level-to-color mapping: `TRACE`/`DEBUG` → `--text-muted`, `INFO` → `--info`, `WARN` → `--warn`, `ERROR` → `--error`. Only the level token itself is colored; the message stays `--text-primary` so colored noise doesn't fight legibility on a busy log stream.

### Two utility classes

- **`.led`** — `text-shadow: 0 0 6px currentColor`. Status indicators read as rack LEDs, glowing in whatever status color they inherit rather than just sitting flat. Paired with `.led-live`, a 2.6s `led-breathe` opacity pulse (1 → 0.55 → 1) for a live connection, behind a `prefers-reduced-motion` guard.
- **`.rule-fade`** — a 1px rule that fades out to the right: `linear-gradient(to right, var(--section, var(--border-subtle)), transparent)` at 50% opacity. A divider that stops shouting partway across the pane instead of running the full width at full strength.

### Two shared components

- **`TabbedPane`** (`src/components/ui/TabbedPane.tsx`) — a pane split into channels, styled like a rack strip. The active tab wears its hue as a 2px top cap; inactive tabs keep theirs at 25% opacity rather than losing it entirely, so the strip reads as a set of channels rather than one accent among greys. It publishes `--section` on the pane root and owns no selection state — the caller decides what's active (settings persists it, the system pane derives it from the route). Used by both the settings window and the system pane.
- **`GroupHeader`** (`src/components/ui/GroupHeader.tsx`) — a section label inside a browse list: a tick, a label, and a count, in the same 10px uppercase used for section headings elsewhere. Sticky, so a long scroll always shows where you are, and opaque rather than blurred — this is a terminal, not a frosted panel.

### Empty states (`src/components/ui/States.tsx`)

`Loading`, `Empty`, and `Failed` — the three things every list view says when it has nothing to show, all rendering at `p-3 text-[12px]`. These had been hand-written in roughly three dozen places and had drifted across five different size/color combinations, so "loading" looked like a different kind of message depending on which pane you were in. `Failed` always routes its error through `errText` rather than interpolating it directly, since a raw IPC error renders as `[object Object]` otherwise.

## Motion

Signal is not a 0ms-by-default, two-exceptions app: `src/styles.css` defines seven keyframe animations, most tied to a specific, purposeful bit of feedback rather than to hover/focus micro-interactions:

| Animation | What it drives | Notes |
|---|---|---|
| `eq-pulse` | Equalizer bars (mini player) oscillating while a track plays | Per-bar duration is staggered inline (`0.55s + i·0.14s`), not fixed in CSS |
| `eq-pulse-soft` | The heart mark's gentler breathing variant | Smaller amplitude (scaleY 1 → 0.8) so the heart silhouette still reads mid-animation; duration also staggered per instance |
| `marquee-x` | Overflowing one-line labels in the mini player | 9s, ease-in-out, infinite alternate |
| `panel-settle` | Stats panels settling in on mount (`opacity`/`translateY`) | 260ms ease-out, staggered by `calc(var(--i, 0) * 45ms)` |
| `meter-sweep-x` | Horizontal meters sweeping from zero once on mount | 520ms, `cubic-bezier(0.2, 0.9, 0.2, 1)`, staggered by `var(--i) * 22ms` |
| `meter-sweep-y` | Vertical meters sweeping from zero once on mount | Same easing/duration as `meter-sweep-x`, staggered by `var(--i) * 18ms` |
| `led-breathe` | `.led-live` — a live connection's LED pulsing | 2.6s ease-in-out infinite, opacity 1 → 0.55 → 1 |

`ease-out` (or the shared `cubic-bezier(0.2, 0.9, 0.2, 1)` used by the meter sweeps) covers everything — nothing gets a spring or a bounce. There are four separate `prefers-reduced-motion: reduce` blocks in the stylesheet, one per animation family (`.eq-bar`/`.eq-bar-soft`, `.marquee > span`, `.panel-settle`/`.meter-x`/`.meter-y`, `.led-live`), and each sets `animation: none` rather than shortening the duration — reduced motion means no motion, not less motion, the same principle the original single-transition version of this section stated, just implemented across a much larger surface than "two exceptions."

## Iconography

Glyph-only. Lucide is not a dependency of this app — every icon need is met with a text glyph rendered for free in the monospace font: `▶` `⏸` `⏭` `⏮` `♥` `⚡` (bit-perfect) and similar cover transport and status needs, colored via `currentColor` so they inherit whichever text token they're placed against rather than carrying their own color. Where a case doesn't have an obvious glyph (settings, folder, drag handle, playlist, device), Signal still reaches for a glyph rather than pulling in an icon library — there is no icon-component escape hatch in the current implementation.

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
