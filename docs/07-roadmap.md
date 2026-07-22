# Roadmap

This document lays out the path from an empty repository to a shippable v1 of Signal, broken into six milestones (M0–M5). Each milestone has a goal, a concrete scope checklist tied to real crate and component names, explicit exit criteria you can verify by using the app, and a rough size estimate. Milestones are meant to be sequential and each one should leave the app in a runnable, demoable state — no long-lived "everything is broken" branches.

Sizes are rough calendar effort for a single focused contributor, not story points: **S** = a few days, **M** = one to two weeks, **L** = three-plus weeks.

## At a Glance

| Milestone | Focus | Primary crates/components touched | Size |
|---|---|---|---|
| M0 | Walking skeleton | workspace scaffold, `signal-core`, `src-tauri`, CI | S |
| M1 | Library | `signal-db`, `signal-scanner`, artwork protocol | L |
| M2 | Playback | `signal-player`, libmpv, inspector pane | L |
| M3 | Core UX | queue, gapless, keyboard layer, palette, `signal-search` | L |
| M4 | Hi-Fi + polish | ReplayGain, exclusive mode, smart playlists, stats, logs, fs watcher | L |
| M5 | Extensibility | `signal-plugins`, MPRIS, Last.fm, packaging | M |

Each milestone assumes every prior milestone's exit criteria still hold — this is a stack, not a set of parallel tracks. A given milestone should not start in earnest until the previous one's exit criteria can be demonstrated by actually using the app, not just by reading a diff.

Everything explicitly listed as a non-goal for Signal — streaming services, cloud sync, recommendation engines, social features, podcasts, video, DRM — stays out of scope for the entire M0–M5 range. If a milestone's scope ever seems to imply one of these, that's a sign the scope item needs to be rewritten, not that the non-goal has quietly changed.

## M0 — Walking Skeleton

**Goal:** Get the full toolchain wired end-to-end. A Tauri window opens, the Rust workspace has its final crate layout (even if most crates are stubs), the frontend shell renders an empty three-pane layout, and CI enforces formatting, linting, and tests from day one. No real features yet — this is the scaffold everything else is built on top of, and it should be boring and solid.

Scope:
- [ ] Cargo workspace with `signal-core`, `signal-db`, `signal-scanner`, `signal-player`, `signal-search`, `signal-plugins` crates (stub implementations where needed) plus the `src-tauri` app crate
- [ ] `signal-core` domain types: `Track`, `Album`, `Artist`, `TrackTechnical`, `PlaybackState`, and the `SignalEvent` enum backed by a `tokio::sync::broadcast` bus
- [ ] Tauri v2 app boots to a single window, dark theme only (no light-mode toggle, no theme system to build)
- [ ] `src/` frontend scaffold: React + TypeScript + Tailwind + shadcn/ui components, TanStack Router and Query wired in, Zustand store initialized
- [ ] Static three-pane layout (sidebar / list / detail-or-inspector) with placeholder content and visible keyboard focus rings
- [ ] GitHub Actions CI: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, frontend `tsc --noEmit` + lint, running on every PR and on push to main
- [ ] `tracing` + `tracing-subscriber` wired in, structured logs to stdout in dev builds
- [ ] `migrations/` directory created (empty placeholder migration) so the schema pipeline exists before M1 needs it

**Exit criteria:** You can clone the repo, run one command, and get a dark Tauri window with three empty panes and working keyboard focus. CI is green on a trivial PR that only touches a comment.

**Size:** S

---

## M1 — Library

**Goal:** Turn a folder of audio files on disk into a browsable library inside the app. This is where `signal-db` and `signal-scanner` earn their names, and where the first non-trivial IPC surface appears.

Scope:
- [ ] `migrations/` SQLite schema: `tracks`, `albums`, `artists`, `artwork` tables, plus an FTS5 virtual table created but left unpopulated (indexing lands in M3)
- [ ] `signal-db`: sqlx models and queries, connection pool setup, migration runner executed on app startup
- [ ] `signal-scanner`: recursive one-shot directory walk, tag extraction via `lofty` across FLAC/ALAC/WAV/AIFF/AAC/MP3/OGG/Opus, dedupe by path + content hash
- [ ] IPC commands: `library_scan`, `library_get_albums`, `library_get_artists`, `library_get_tracks`
- [ ] `scanner:progress` event streamed to the UI during a scan (files processed / total, current path)
- [ ] Artwork pipeline: extract embedded tag art, fall back to `folder.jpg` / `cover.png` in the track's directory, serve images to the frontend through a custom `signal-art://` asset protocol handler registered in `src-tauri`
- [ ] UI: scan trigger (first-run flow or a settings action), album grid view, artist list view, album detail view with track listing
- [ ] Scanner is resilient to bad input: unreadable files and corrupt tags are logged and skipped, a single bad file never aborts the whole scan

**Exit criteria:** You can point Signal at a folder of FLAC/MP3 files, trigger a scan, watch progress update live, and end up with correctly tagged albums (with artwork), artists, and track listings browsable in the UI.

**Size:** L

---

## M2 — Playback

**Goal:** Make Signal actually play audio, and surface real technical data about what's playing rather than placeholder text. This is the milestone where `signal-player` and libmpv integration land.

Scope:
- [ ] `signal-player`: libmpv embedding (bindings crate or raw FFI), one persistent mpv instance owned by the player crate
- [ ] IPC commands: `player_play`, `player_pause`, `player_stop`, `player_seek`, `player_set_volume`, `player_next`, `player_previous`
- [ ] `player:state` event (playing/paused/stopped + current track), `player:progress` event (position/duration, throttled to a few updates per second, not every mpv tick)
- [ ] UI: transport controls (play/pause, seek bar, volume), now-playing bar driven entirely by real events, not local optimistic state
- [ ] Audio inspector pane populated with real `TrackTechnical` data: codec, sample rate, bit depth, bitrate, channel count — sourced from `lofty` tag metadata and cross-checked against what mpv reports it's actually outputting
- [ ] Format coverage smoke tests: one fixture file per supported format decodes successfully and reports the expected technical data
- [ ] Missing/moved files are handled gracefully: a track whose file can't be found is marked missing in the UI and skipped on play rather than crashing playback

**Exit criteria:** You can click a track in the library, hear it play through your speakers, control it with the transport (including seeking), and the inspector pane shows accurate codec/sample-rate/bit-depth information for whatever is currently playing.

**Size:** L

---

## M3 — Core UX

**Goal:** This is the milestone that makes Signal feel like "the LazyGit of music players" — a real queue with git-staging semantics, gapless playback, a full keyboard layer, a command palette, and instant full-text search. Before this milestone Signal plays music; after it, Signal is fast to *use*.

Scope:
- [ ] Queue: `queue_add`, `queue_remove`, `queue_move`, `queue_clear` IPC commands with git-staging-style semantics (tracks can be staged into the queue and reordered before committing to play order); `queue:changed` event; queue state persisted to SQLite so it survives app restarts
- [ ] Gapless playback implemented via mpv's own playlist window (pre-loading the next track into mpv's internal playlist) rather than a manual stop/reload cycle, which is where audible gaps come from
- [ ] Centralized keyboard layer: `j`/`k` list navigation, `space` play/pause, `/` focuses search, `gg`/`G` jump to top/bottom of a list, `Tab` cycles panes — implemented as one keymap dispatcher, not scattered per-component `onKeyDown` handlers
- [ ] Command palette v1: fuzzy-matched command list, `palette_execute` IPC command, initial command set covering playback control, navigation (go to album/artist), triggering a library scan, and toggling the inspector
- [ ] `signal-search`: FTS5 index populated and kept in sync with `signal-db` content, `search_query` IPC command, query language v1 supporting field filters (`artist:`, `album:`, `year:`) alongside bare full-text terms
- [ ] UI: search results view wired to `search_query`, palette overlay component with keyboard-only operation, queue panel visually distinguishing staged vs. active queue state

**Exit criteria:** You can operate the entire app without touching the mouse — search for a track with `/`, add it to the queue, reorder the queue with keyboard shortcuts, open the command palette with a shortcut and run a command by name, and play two consecutive tracks from the same album with no audible gap between them.

**Size:** L

---

## M4 — Hi-Fi + Polish

**Goal:** Deliver on the "Hi-Fi" half of the product promise, and add the operational tooling (statistics, logs, filesystem watching) that turns Signal from a demo into something you'd actually run as your daily player.

Scope:
- [ ] ReplayGain: track and album gain modes, values read from tags where present and computed where absent, applied through mpv's volume/filter chain, with a UI toggle to switch modes or disable
- [ ] `device_list` IPC command and an output device picker in the UI; switching devices does not require an app restart
- [ ] Exclusive mode per OS (WASAPI exclusive on Windows, Core Audio hog mode on macOS, direct ALSA/PipeWire access on Linux) with automatic sample-rate switching to match the source file's native rate
- [ ] Bit-perfect indicator in the now-playing bar / inspector, showing whether the current stream is playing exclusive and unresampled, or has fallen back to shared/resampled output
- [ ] Smart playlists: rule builder over genre, year, rating, play count, and date-added, backed by saved queries against `signal-db`/`signal-search`; static (manually curated) playlists supported alongside smart ones
- [ ] Stats v1: `play_events` table (`track_id`, `started_at`, `ms_played`, `completed`), `stats_overview` IPC command, a stats view with uPlot-based listening-history charts and a heatmap
- [ ] In-app log viewer: tails `tracing` output, `log:line` event, filterable by level and module, reachable as its own pane or via the command palette
- [ ] Filesystem watcher (`notify` crate) driving incremental library updates — new, removed, and renamed files are reflected without triggering a full rescan

**Exit criteria:** You can play a file bit-perfect in exclusive mode with the indicator confirming it, build a smart playlist from a rule (e.g. "genre:jazz, rating >= 4"), see your listening history rendered as a heatmap, and watch the library update automatically the moment you drop a new file into a watched folder.

**Size:** L

---

## M5 — Extensibility

**Goal:** Ship the plugin host described in [08-plugins.md](./08-plugins.md), prove it out with two real plugins, and get Signal into a distributable, installable state on all three desktop platforms.

Scope:
- [ ] `signal-plugins` host: `SignalPlugin` trait implementation, `PluginContext` (DB reader, event bus subscription, namespaced config storage, command registrar), each plugin gated behind its own Cargo feature flag
- [ ] MPRIS plugin (Linux): maps `SignalEvent` to the `org.mpris.MediaPlayer2` D-Bus interface via `zbus`
- [ ] Last.fm scrobbler plugin: auth token flow, scrobble logic driven by `TrackEnded` implementing the 50%-or-4-minute rule, offline queue for scrobbles that fail to submit
- [ ] Settings UI: plugin list with enable/disable toggles and per-plugin configuration forms
- [ ] Palette commands for plugin control: `plugin enable <name>`, `plugin disable <name>`, `plugin list`
- [ ] Packaging via the Tauri bundler: `.dmg` for macOS, `.msi` for Windows, `.AppImage` for Linux, with code signing where feasible on each platform
- [ ] Docs site rendered from `docs/`, covering installation, keybindings, the search query language, and a plugin authoring guide

**Exit criteria:** You can download and install Signal from a `.dmg`/`.msi`/`.AppImage`, enable the Last.fm plugin from settings, authenticate, and see real scrobbles land on your Last.fm profile. On Linux, Signal shows up correctly in an MPRIS-aware system tray and responds to system media key controls.

**Size:** M

---

## Risk Register

Top five technical risks, ranked roughly by combined likelihood and impact, with mitigations.

| # | Risk | Impact | Mitigation |
|---|------|--------|------------|
| 1 | **libmpv static linking / bundling per OS.** macOS and Windows need libmpv and its ffmpeg dependencies bundled, signed, and notarized; Linux relies on a system-installed libmpv whose version and build flags vary by distro. | Playback simply doesn't work on a subset of user machines, or code-signing/notarization blocks releases. | Vendor a pinned libmpv build per platform through a CI build matrix rather than trusting `brew`/system packages; smoke-test actual audio playback on each OS in CI, not just compilation; document the minimum system libmpv version required on Linux and detect/warn on startup if it's too old. |
| 2 | **Exclusive-mode audio quirks per OS.** WASAPI exclusive mode, Core Audio hog mode, and ALSA/PipeWire direct access all have different failure modes, and behavior varies further by audio driver and hardware (USB DACs, Bluetooth, virtual devices). | Exclusive mode silently fails, glitches, or locks the device on some hardware, undermining the "Hi-Fi" pitch. | Treat exclusive mode as best-effort with an explicit, visible fallback to shared mode rather than a hard failure; build a real-hardware test matrix (at minimum: built-in output, one USB DAC, one Bluetooth device per OS) exercised before each M4-related release; surface the bit-perfect indicator so users can tell when fallback happened instead of assuming silence means success. |
| 3 | **FTS5 ranking quality.** Naive SQLite FTS5 bm25 ranking handles exact substring matches fine but does poorly with typos, diacritics, "The" artist prefixes, and other real-world query noise users expect a Spotlight-like search to absorb. | Search feels broken even when the data is correct, which is especially damaging for a "keyboard-first, search-centric" product. | Use the `unicode61` tokenizer with diacritics removal from the start; maintain a small alias/normalization table for common patterns (leading "The", featuring artists, etc.); build a logged test set of real queries against a real library and tune ranking against it iteratively rather than shipping bm25 defaults untouched; keep the M3 query language as an escape hatch for users who want exact field matches. |
| 4 | **Gapless playback + queue resync complexity.** mpv's playlist-window model for gapless playback assumes a relatively static internal playlist, but Signal's git-staging-style queue can be reordered or edited by the user mid-playback. Keeping mpv's internal playlist and Signal's queue state consistent without race conditions is genuinely hard. | Gapless breaks intermittently, or worse, playback order silently diverges from what the queue UI shows. | Make Signal's queue the single source of truth and treat mpv's internal playlist as a derived cache that gets rebuilt whenever it diverges, rather than trying to keep two independent state machines in lockstep; write integration tests specifically for "reorder queue while a gapless pair is mid-transition" scenarios before M3 is called done. |
| 5 | **Tauri IPC performance on very large libraries (100k+ tracks).** List-returning IPC commands (`library_get_albums`, `library_get_tracks`, search results) that serialize and transfer full result sets over the IPC bridge can cause visible UI stutter or long first-paint times as library size grows. | Signal feels sluggish precisely for the power users most likely to have huge, meticulously tagged libraries — the target audience. | Design every list-returning IPC command with pagination/virtualization from M1 onward instead of retrofitting it later; never send a full-library snapshot over IPC in one call; add a synthetic 100k-track fixture library to CI and benchmark list/search command latency against it as a standing check. |

## Definition of Done, Per Milestone

A milestone is not "done" when the feature works on the author's machine. The following bar applies at every milestone before it's tagged:

- **Tests.** Unit tests for non-trivial logic in `signal-core`, `signal-db`, `signal-scanner`, and `signal-search`. `signal-player` gets tests wherever mpv interaction can be abstracted behind a trait for mocking; the parts that can't be mocked (actual audio output) are covered by the manual smoke checklist instead. Scanner and format-related work is covered by fixture-based tests using real sample files per supported codec.
- **Tracing spans.** Every IPC command handler is wrapped in a `tracing` span capturing its arguments and duration. `signal-scanner` emits a span per file processed. `signal-player` emits a span per transport action. This isn't optional polish — it's what makes the M4 in-app log viewer and future debugging useful, so it needs to exist from the milestone that introduces each subsystem, not retrofitted later.
- **No clippy warnings.** `cargo clippy -- -D warnings` passes cleanly in CI on every merge to main, not just at milestone boundaries. A milestone cannot be tagged done if CI is red or warnings have been suppressed with blanket `#[allow]`s.
- **Frontend hygiene.** TypeScript strict mode passes with no `any` escapes introduced for convenience, and no uncaught console errors during normal navigation through the milestone's new UI.
- **Manual smoke test.** Before a milestone is tagged, run its exit-criteria scenario by hand on macOS, Windows, and Linux at least once. Milestones M2 and M4 in particular (audio output, exclusive mode) cannot be verified by CI alone and need real hardware.

Milestones build on each other's quality bar cumulatively — M3's CI must still pass all of M1 and M2's tests, not just its own.
