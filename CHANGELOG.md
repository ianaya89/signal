# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Updates could not be installed from the UI. Clicking the version chip fired
  the download immediately and every failure was invisible: repeat clicks
  started parallel downloads, a missing content-length froze the label on
  `updating…`, an expired update handle dead-ended on a toast, and a failed
  relaunch showed nothing. The chip now opens a review dialog — version diff,
  release notes, live progress, and errors that stay on screen — and the
  install is single-flight, recovers an expired handle by re-checking, and
  reports a failed restart.
- The doctor's online artwork lookup looked frozen. It is slow by protocol —
  MusicBrainz allows one request per second, so a 15-album batch takes half a
  minute at best — and the button gave no feedback until the whole run
  finished, with per-album failures only reaching the log file. The run now
  streams every album's verdict to a live console with a progress bar, an
  estimate and a stop button, and reports why a lookup failed (rate limited,
  offline, no cover on the archive).
- Album or artist names containing `:`, `/` or quotes made MusicBrainz reject
  the search outright, which surfaced as "no cover found". Queries are now
  escaped, and a loose search retries when the exact phrase misses.
- Scanning a folder with an interrupted download reported every placeholder as
  a corrupt file (`tag parse: Flac: File missing "fLaC" stream marker`). Files
  that are still downloading (a sibling `.part`, `.crdownload`, `.download`…),
  iCloud files that were never materialized, and zero-filled placeholders are
  now skipped with a reason in the log instead of counted as import errors.
- Toasts were rendered behind the transport dock.

## [0.1.3] - 2026-07-28

### Added

- Favorites view. Everything you marked in one place: `♥` favorites and `✦`
  4-star-and-up ratings, filterable (`all` / `♥` / `✦`), reachable from the
  library nav, `F`, and the command palette.
- Keyboard cursors in the views that lacked them — the album grid (`j`/`k`
  step a row, `h`/`l` a card), artists, genres, playlists, search results and
  discover.

### Changed

- Transport and status swapped places. Playback controls, timeline, modes and
  volume now sit in a dock along the bottom; the status strip took over the
  top bar and keeps the window-drag region.
- The stats view was rebuilt as an instrument panel: hero readouts (plays,
  hours, distinct tracks, listening streak), bracketed panels, meters that
  sweep from zero, a heatmap with month ticks, an hour clock with its peak
  flagged, a weekday histogram, top tracks, codec meters carrying the
  lossless/lossy semantics, and a library summary linking into favorites.

### Fixed

- Tracks rated 4 stars or higher appeared in no list — the heart flag and the
  rating are separate signals and only the first was listed anywhere.

## [0.1.2] - 2026-07-27

### Added

- In-app updates. signal checks for a new release on launch and offers it in
  the status bar, the command palette and settings. The launch check is
  silent and can be turned off. `.deb` installs are left to apt.
- Update artifacts are signed: the macOS `.app.tar.gz` and the Linux AppImage
  each ship a minisign signature verified before anything is installed.
- Arrow key navigation in lists — `↓`/`↑` mirror `j`/`k`, `Home`/`End` mirror
  `gg`/`G`.

### Fixed

- The status bar showed a hardcoded version instead of the running one.

## [0.1.1] - 2026-07-27

### Changed

- The app is named `signal` in lowercase everywhere: `signal.app`, the binary
  inside it, and the release assets (`signal_0.1.1_arm64.dmg`). The installer
  still handles bundles named `Signal.app` from earlier releases.

### Fixed

- macOS: the app crashed at launch with `Library not loaded: libmpv.2.dylib
  (duplicate LC_RPATH)` on macOS 15+, where dyld rejects a Mach-O that carries
  the same rpath twice.
- Linux: the AppImage now detects libmpv's soname instead of assuming
  `.so.2`, so builds on hosts shipping `libmpv.so.1` work.
- CI installs `libmpv-dev` and splits the Linux release runners.

## [0.1.0] - 2026-07-26

First pre-release.

### Added

- **Library** — multi-root scanning with a filesystem watcher, per-folder
  removal and exclusions, FTS5 search, multi-genre support, various-artists
  detection, m3u import, cover-art fetch, a metadata editor with tag
  write-back, and album/track rename.
- **Playback** — libmpv backend (FLAC, ALAC, WAV, AIFF, AAC, MP3, OGG, Opus)
  with gapless, ReplayGain, exclusive mode, automatic sample-rate switching,
  bit-perfect detection, output device picker, and a queue with staging,
  reordering and save-as-playlist.
- **Organise** — playlists and smart playlists, like/love ratings, favorites,
  discover shelves computed from listening history, and a stats view.
- **Maintenance** — a doctor view that finds missing files, relinks moved
  ones, resolves duplicates and prunes dead entries; plus database backup.
- **Interface** — keyboard-first normal mode, command palette, a `?` overlay
  generated from the binding registry, three-pane layout with an audio-chain
  inspector, mini player and floating pulse mode, context menus, toasts,
  virtualized lists, and dark/light themes.
- **Integration** — OS media keys and Now Playing, session resume,
  ListenBrainz scrobbling, a Unix control socket with the `signal` CLI on top,
  and `config.toml` applied live.
- **Distribution** — macOS `.dmg` with libmpv bundled, Linux `.deb` and
  AppImage, an install script, and a GitHub Pages landing page.

[Unreleased]: https://github.com/ianaya89/signal/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/ianaya89/signal/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/ianaya89/signal/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/ianaya89/signal/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ianaya89/signal/releases/tag/v0.1.0
