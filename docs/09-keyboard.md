# Keyboard Interaction Model

Signal is built keyboard-first: every action reachable by mouse must also be reachable by keyboard, but the reverse is not required. This document is the source of truth for the actual key bindings, the mode stack that governs how a keypress gets interpreted, the precedence rules that resolve conflicts, and the rebinding/accessibility contract. The frontend architecture that implements this model (the mode-stack store, pane focus registry, binding dispatcher) lives in `06-frontend.md`; this document defines *what* it must do, not how the React code is organized.

## Principles

- **Vim-influenced, not vim-cloned.** `j`/`k` navigation, `gg`/`G`, and a normal-mode-first design borrow vim's muscle memory because a large share of Signal's target users already have it. Signal is not a modal text editor, though — there is no insert-mode-for-everything, and most actions are single keys, not vim-style operator+motion combinations.
- **Single-key actions in normal mode.** Every common action is one keypress. No binding in normal mode requires holding three or more keys simultaneously (a true chord). Two-key *sequences* typed in succession — `gg`, `r` then a digit — are allowed and documented separately in the section below; they are not chords, since the keys are pressed one after another, not together.
- **Every visible action shows its hint.** Any button, row action, or menu entry that has a keyboard equivalent renders that binding next to it (a `kbd`-styled chip). Nothing is a keyboard-only secret except a small set of power-user sequences, and those are fully documented in the `?` help overlay.
- **Every binding is rebindable.** Defaults ship in code, but the effective binding set is a JSON file (`keybindings.json`) the user can edit directly or through a settings UI. Nothing is hardcoded in a way that bypasses that file — see "Rebinding" below.
- **Consistent verbs across panes.** `j`/`k` always move, `Enter` always opens/activates, `a` always adds/stages, `x` always removes, `o` always reveals in the OS file manager, `y` always yanks to clipboard. A verb never means one thing in the library and a different thing in the queue; where a pane needs a variant (e.g. queue's `x` unstages rather than deleting a file), the underlying intent — "remove this from the current context" — stays the same.

## Mode stack

Signal has exactly five modes: one default (`NORMAL`) and four that temporarily take over key handling (`PALETTE`, `SEARCH`, `INPUT`, `HELP`). At any moment exactly one mode is active; there is no true nesting stack of arbitrary depth, but `NORMAL` is always the mode every other mode returns to, so it behaves like the bottom of a stack.

```
                  ┌────────────────────────────────────────────────┐
                  │                     NORMAL                      │
                  │   default mode; global bindings + the focused    │
                  │        pane's scoped bindings are active         │
                  └───┬───────────┬────────────┬────────────┬──────┘
             Ctrl+P    │      /   │      :,e,w  │        ?  │
                       ▼          ▼            ▼            ▼
                 ┌──────────┐┌──────────┐┌──────────┐┌──────────┐
                 │ PALETTE  ││  SEARCH  ││  INPUT   ││  HELP    │
                 │ overlay, ││ overlay, ││ inline   ││ overlay, │
                 │ fuzzy    ││ live-    ││ text     ││ read-    │
                 │ command  ││ filtered ││ field    ││ only     │
                 │ list     ││ results  ││          ││          │
                 └────┬─────┘└────┬─────┘└────┬─────┘└────┬─────┘
                      │ Esc       │ Esc       │ Esc       │ Esc
                      └───────────┴───────────┴───────────┘
                                       ▼
                                    NORMAL
```

`PALETTE` (Ctrl+P) and `SEARCH` (`/`) are grouped together in the bindings table below because both pair a text input with a navigable result list — Ctrl+n/p, arrows, Enter, Tab all behave the same way in either. `INPUT` is different: it is a plain text field with no result list, entered by `:` (command line), `e` (inline metadata edit), `w` (playlist-name prompt), and any other rename/edit affordance. `HELP` is read-only: it renders the live binding registry and accepts only scroll/`j`/`k` and an inline filter, never a write action.

Note that `r` followed by a digit (set rating) is **not** a mode change. It stays entirely inside `NORMAL` mode as a multi-key sequence handled by the state machine described later in this document — there is no dedicated "rating" mode, because the interaction is short-lived, has no free text, and needs no overlay.

### What Esc does per mode

| Mode | Esc behavior |
|---|---|
| `PALETTE` | Dismiss the overlay, discard the filter text, return focus to whatever pane had it before Ctrl+P was pressed. |
| `SEARCH` | Dismiss the search panel; if a query was already committed (Enter was pressed earlier), the committed results stay applied to the view — Esc only cancels *in-progress* edits to the query, it does not clear an already-applied filter (that needs a second Esc, see below). |
| `INPUT` | Cancel the edit or prompt, discard any uncommitted text, return to `NORMAL` without applying changes. |
| `HELP` | Close the overlay, return to `NORMAL`. |
| `NORMAL` | Cascades through a fixed precedence, innermost first: (1) clear an active multi-select/visual-select range, else (2) clear a currently-applied search filter on the focused view, else (3) close the Inspector detail pane if it is showing something other than "now playing" (e.g. a right-click preview), else (4) defocus back to the last-focused Library pane. Esc in `NORMAL` never quits the app and never stops playback. |

### Global vs. mode-scoped keys

Global bindings (playback, volume, pane navigation, view switches) are only live while in `NORMAL` mode. The moment any other mode is active, an editable field or an overlay owns the keyboard and global bindings are suspended — this is deliberate: typing a literal space into the search box must insert a space character, not toggle playback. `Esc` is the single exception: it is universally live in every mode, guaranteed to always do something, and always moves the mode stack one level back toward `NORMAL`.

## Global bindings

Active in `NORMAL` mode, regardless of which pane has focus.

| Key | Action |
|---|---|
| `Space` | Play / pause |
| `Ctrl+P` | Open command palette |
| `/` | Open search |
| `:` | Focus command line |
| `?` | Toggle help overlay |
| `Tab` / `Shift+Tab` | Cycle pane focus forward / backward |
| `1` / `2` / `3` | Jump focus to Library / Center / Right rail (Inspector+Queue) |
| `m` | Mute toggle |
| `+` / `-` (or `=` / `-`) | Volume up / down (`=` doubles as `+` so it works without Shift on US layouts) |
| `[` / `]` | Seek −5s / +5s |
| `{` / `}` | Previous / next track |
| `i` | Toggle Inspector visibility |
| `q` | Toggle queue focus — jump into the queue list inside the right rail; press again or `Tab` away to leave |
| `L` | Open Logs view (swaps the Center pane to the log viewer) |
| `S` | Open Stats view (swaps the Center pane to listening-history/stats) |
| `Esc` | Context-dependent "back" — see table above |

## List/table bindings (any pane with a scrollable list)

Applies to Library nav, the track table, playlist lists, search results, and the queue list, on top of that pane's own scope-specific bindings below.

| Key | Action |
|---|---|
| `j` / `k` | Move down / up |
| `↓` / `↑` | Move down / up — same as `j`/`k`, for hands that never learned vim |
| `gg` | Jump to top (sequence, see below) |
| `G` | Jump to bottom |
| `Home` / `End` | Jump to top / bottom |
| `Ctrl+d` / `Ctrl+u` | Half-page down / up |
| `Enter` | Open / play — expands a tree node in Library, plays a track in a track table, jump-plays in the queue |
| `o` | Open containing folder in the OS file manager |
| `x` | Remove — context-dependent: removes from a playlist in playlist views, unstages in the queue pane (see Queue bindings) |

**Note on Space:** because `Space` is reserved globally for play/pause, list views never repurpose bare `Space` for row selection or multi-select the way some file managers do. Where multi-select is genuinely needed (bulk-add to queue, bulk playlist removal), `v` enters visual-select mode — extend the range with `j`/`k`, confirm with the action key you want applied (`a` to stage all, `x` to remove all), cancel with `Esc` — following vim's visual-mode convention instead of overloading `Space`.

## Library / Center bindings

Applies when focus is on a Library nav item or a track row in the Center pane.

| Key | Action |
|---|---|
| `a` | Add to queue — "stage", the git-add metaphor: the track/album is queued but nothing about the current view changes |
| `A` | Play album next — inserts the whole album directly after the currently playing track |
| `f` | Toggle favorite |
| `r` then `1`–`5` | Set rating (sequence, see below) |
| `e` | Edit metadata (opens an inline field set, enters `INPUT` mode) |
| `y` | Yank file path to clipboard |

## Queue pane bindings

Applies when focus is inside the queue list (reached via `q` or by tabbing into the right rail's queue section).

| Key | Action |
|---|---|
| `J` / `K` | Move the selected item down / up in the queue order |
| `x` | Unstage — remove from the queue |
| `c` | Clear the entire queue |
| `w` | Save the current queue as a playlist ("write"; enters `INPUT` mode for the playlist name) |
| `Enter` | Jump-play — start playback at the selected queue item |

`J`/`K` here are queue-scope bindings, distinct from the list-scope `j`/`k` (lowercase) which only move the selection cursor; uppercase reorders. This mirrors the same shift-to-reorder convention used in most vim-adjacent list UIs.

## Palette / Search bindings

Applies while `PALETTE` or `SEARCH` mode is active. Both modes share this table because they share the same input+list interaction shape.

| Key | Action |
|---|---|
| `Ctrl+n` / `Ctrl+p` (or `↓` / `↑`) | Navigate results |
| `Enter` | Execute the highlighted palette command, or commit/open the highlighted search result |
| `Tab` | Complete — accepts the highlighted palette entry's text, or completes a `field:` token in search |
| `Esc` | Dismiss, return to `NORMAL` |

All other keystrokes in these modes go to the text buffer as literal characters — see the precedence algorithm below for exactly how that passthrough is decided.

## Precedence and conflict resolution

Every keydown event is resolved by a fixed, numbered algorithm. Handlers are checked in this order and the first one that claims the key stops evaluation:

1. **Editable input focus check.** If a genuine editable DOM element has focus (the palette/search text box, the `:` command line, an inline rename/metadata field), that field owns the event first. It always keeps literal printable characters. It only lets `Esc` and — where the field explicitly opts in, as PALETTE/SEARCH inputs do — `Enter`, `Tab`, `Ctrl+n`, `Ctrl+p`, `↑`, `↓` pass through to the app's key layer for the reasons in the table above. A plain `INPUT`-mode field (rename, playlist name) opts into nothing beyond `Esc`/`Enter`/`Tab`.
2. **Mode handler.** If no field consumed the key and a non-`NORMAL` mode is active (`PALETTE`, `SEARCH`, `HELP`), that mode's dedicated handler receives the key next. It only recognizes its own scoped bindings plus `Esc`. If it doesn't recognize the key, the key is dropped — global and pane bindings never leak through an open overlay, so `Space` does nothing while the palette is open.
3. **Pane handler.** If we are in `NORMAL` mode, the currently focused pane's scoped handler receives the key first — this includes the list-scope table and whichever of Library/Queue's more specific bindings apply to that pane. A pane-scoped binding always wins over an identically-keyed global binding (see the `x` example: Queue's "unstage" shadows the generic list-scope "remove" when the queue has focus, and Library's `a` "stage" has no global counterpart to conflict with in the first place).
4. **Global handler.** If the pane handler didn't claim the key, the global table is checked.
5. **No-op.** If nothing claims the key, it is dropped silently in production. In development builds it is logged at `DEBUG` so binding coverage gaps show up during testing.

This is the concrete meaning of "mode handlers before pane handlers before global": modes are checked first because they represent an explicit, user-initiated override of normal browsing; panes are checked next because a pane's own semantics (e.g. queue reordering) are more specific than the app-wide defaults; the global table is the fallback everyone else can rely on.

## Multi-key sequences

Two bindings are true sequences rather than single keys: `gg` (jump to top) and `r` + digit (set rating). Both are handled by the same small state machine, scoped per pane instance so that starting a sequence in one pane and clicking into another doesn't carry a stale pending key across.

```
                              timeout (500ms) elapses,
                              or a non-continuation key arrives:
                              abort — return to IDLE, then
                              reprocess that key normally
                     ┌───────────────────────────────────────────┐
                     │                                            │
                     ▼                                            │
        ┌────────┐  trigger key   ┌──────────────┐  continuation  │
        │  IDLE  │ ─────────────► │   PENDING     │ ─────key────► EXECUTE, back to IDLE
        └────────┘   (g or r)     │ (0.0–0.5s)    │  (see below)
             ▲                    └──────────────┘
             └──────────────────────────────────────────────────────┘
                                     (after EXECUTE)
```

- **`gg`:** trigger key `g` moves `IDLE → PENDING`. The only continuation that completes it is a second `g` within 500ms, executing "jump to top". Any other key, or the timeout, aborts back to `IDLE`; the key that caused the abort is then reprocessed as if the sequence had never started (so `g` then `j` cancels the pending top-jump and *also* moves the cursor down one row — no keystroke is silently eaten).
- **`r` + digit:** trigger key `r` moves `IDLE → PENDING`. Continuations `1`–`5` complete it, executing "set rating to N". Any other key (including `0`, `6`–`9`, or a letter) aborts and is reprocessed normally.

The 500ms window is a single shared constant (`sequenceTimeoutMs` in the binding dispatcher config), not per-binding, so the whole system behaves predictably regardless of which sequence is in flight.

## Discoverability

- The `?` help overlay renders the **live binding registry**, not a static doc page — it reads the same effective bindings (defaults merged with the user's `keybindings.json` overrides) that the dispatcher uses, grouped by scope (Global, List, Library, Queue, Palette/Search). Rebind a key and the help overlay reflects it immediately.
- The bottom status bar shows **contextual hints** for the currently focused pane, LazyGit-style — e.g. focused on the track table it shows `j/k move · Enter play · a stage · f fav · r rate`; focused on the queue it shows `J/K reorder · x unstage · c clear · w save`. This is the single biggest discoverability surface since it's visible at all times without opening anything.
- Every entry in the command palette displays its bound key (if any) right-aligned next to the command name, so browsing the palette itself teaches the shortcuts.

## Rebinding

The effective binding set is `keybindings.json`, loaded from the app's config directory at startup and hot-reloaded on save. It is a flat map of action id to key string (or an array of key strings, for actions with more than one valid binding, like Palette navigation accepting both `Ctrl+n` and `↓`):

```json
{
  "global.playPause": "Space",
  "global.commandPalette": "Ctrl+P",
  "global.search": "/",
  "global.commandLine": ":",
  "global.help": "?",
  "global.paneCycleNext": "Tab",
  "global.paneCyclePrev": "Shift+Tab",
  "global.paneJump1": "1",
  "global.paneJump2": "2",
  "global.paneJump3": "3",
  "global.mute": "m",
  "global.volumeUp": "=",
  "global.volumeDown": "-",
  "global.seekBack": "[",
  "global.seekForward": "]",
  "global.prevTrack": "{",
  "global.nextTrack": "}",
  "global.toggleInspector": "i",
  "global.toggleQueueFocus": "q",
  "global.logsView": "L",
  "global.statsView": "S",

  "list.down": "j",
  "list.up": "k",
  "list.top": "gg",
  "list.bottom": "G",
  "list.halfPageDown": "Ctrl+d",
  "list.halfPageUp": "Ctrl+u",
  "list.open": "Enter",
  "list.openFolder": "o",
  "list.remove": "x",

  "library.addToQueue": "a",
  "library.playNext": "A",
  "library.toggleFavorite": "f",
  "library.setRating": "r",
  "library.editMetadata": "e",
  "library.yankPath": "y",

  "queue.moveDown": "J",
  "queue.moveUp": "K",
  "queue.unstage": "x",
  "queue.clear": "c",
  "queue.saveAsPlaylist": "w",
  "queue.jumpPlay": "Enter",

  "palette.navigateDown": ["Ctrl+n", "Down"],
  "palette.navigateUp": ["Ctrl+p", "Up"],
  "palette.execute": "Enter",
  "palette.complete": "Tab",
  "palette.dismiss": "Esc"
}
```

### Validation rules

- **Unknown action ids are rejected at load.** The binding registry is fixed at compile time (every action Signal supports has a static id); a `keybindings.json` entry referencing an id that doesn't exist is ignored, logged at `WARN`, and that action falls back to its built-in default.
- **Key strings are parsed by a canonical parser** that normalizes modifier order (`Ctrl+Shift+K`, never `Shift+Ctrl+K`) and rejects malformed combinations (bare modifier with no key, unknown key names) at load time, not at keypress time.
- **Collision detection is scoped, not global.** Two actions cannot share a key within the *same* scope — e.g. binding both `global.mute` and `global.toggleInspector` to `i` is rejected at load with an error naming both conflicting action ids, and both fall back to defaults until fixed. Two actions in *different* scopes are allowed to share a key, because only one scope's handler is ever live for a given keypress (see precedence above) — `queue.unstage` and `list.remove` both defaulting to `x` is not a collision, it's the intended shadowing behavior where the more specific queue scope wins while the queue has focus.
- **A handful of functions cannot be fully unbound**, only rebound to a different key: `Esc` as the universal mode-exit, and the command palette's own open action (whatever key it's bound to). This exists purely to prevent a keyboard-only user from configuring themselves into a state with no way back to a known-good mode.
- **Rebinding takes effect live** and is re-validated against the full merged registry before being written to disk. If the on-disk file is hand-edited into an invalid state (bad JSON, an unresolvable collision), Signal falls back to the built-in defaults for the whole file, shows a toast explaining why, and logs the parse/validation error — it never partially applies a broken file.

## Accessibility notes

- **Every binding has a non-keyboard equivalent.** Global and pane-scoped actions are all exposed as entries in the command palette (mouse-clickable, screen-reader-focusable, each with a proper ARIA label and role), and pane-local actions additionally appear in a right-click/long-press context menu. The mouse is optional for a sighted keyboard user, but it is never removed as an assistive-technology path — screen readers navigate via standard focus order and ARIA roles/live regions, not by simulating raw key handling.
- **Mode changes are announced** through an ARIA live region (`aria-live="polite"`) — entering `SEARCH`, `PALETTE`, or `HELP` announces the mode name, and toasts are announced the same way they're shown visually.
- **No default binding collides with an OS-reserved shortcut.** As a category, Signal's defaults use only bare printable characters/punctuation and `Ctrl+<letter>` combinations — never `Alt`, `Cmd`/`Super`, or function-key modifiers — which is exactly the category platforms reserve for window/app switching and system chrome. The specific combinations that were deliberately avoided:

| Platform | Reserved shortcuts avoided | How |
|---|---|---|
| macOS | `Cmd+Space` (Spotlight), `Cmd+Tab`/`` Cmd+` `` (app/window switching), `Cmd+Q/W/M/H` (quit/close/minimize/hide), `Ctrl+←/→` (Spaces) | No default binding uses `Cmd` or `Ctrl+arrow` at all |
| Windows | `Win+*`, `Alt+F4` (close), `Alt+Tab` (switch), `Ctrl+Alt+Del`, `Ctrl+Shift+Esc` (task manager), `Ctrl+Esc` (start menu) | No default binding uses `Alt`, `Win`, or `Ctrl+Alt` |
| Linux (GNOME/KDE defaults) | `Super+*`, `Ctrl+Alt+T` (terminal), `Ctrl+Alt+F1–F7` (VT switch), `Alt+Tab`, `Ctrl+Alt+←/→` (workspace switch) | Same as above — no `Alt`/`Super`/`Ctrl+Alt` bindings exist by default |

A user is still free to *rebind into* one of these combinations via `keybindings.json` — Signal doesn't police that, since a rebind is an explicit choice made with full knowledge of the tradeoff — but the shipped defaults never force that collision on anyone.
