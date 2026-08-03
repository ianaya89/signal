# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5] - 2026-08-03

### Added

- Audio authenticity detector. A new "suspicious audio" section in the doctor
  decodes lossless files and inspects their spectrum for three classic frauds:
  "hi-res" 24/96 files whose content actually stops at CD bandwidth (upsampled),
  FLACs with a lossy encoder's brickwall cliff between 15-20.8 kHz (transcoded
  from MP3/AAC, naming the likely source bitrate), and 24-bit files padded from
  a 16-bit master (reported as info, not an error). Trigger it from the new
  "analyze library" button, the command palette's `doctor: analyze audio
  authenticity`, or the terminal with `signal analyze [force|status]`; the
  scan runs in the background on a small worker pool (2-4 files at once) so
  library-wide runs finish in a fraction of the time, with a streaming
  per-track log, a progress bar and a stop button, and verdicts persist
  across restarts.
  Flagged tracks explain themselves in plain language ("content stops at 21.1
  kHz with a 47 dB cliff…") alongside a confidence percentage — the detector is
  tuned conservative, so gentle analog rolloffs, quiet masters and dither noise
  don't trip it.
- Embedded OpenSubsonic server. A new "mobile server" section in settings lets
  you set a password and press start; any Subsonic client on the LAN — Symfonium
  (Android), Amperfy or play:Sub (iOS), Feishin (desktop) — can then browse and
  stream the library at `http://<machine-ip>:port` (default 4040). Browsing,
  search, playlists and covers all work, seeking works via HTTP Range, and stars
  or ratings set from a phone land back in Signal; a track played to completion
  on the phone counts toward Signal's stats (play source `remote`). Smart
  playlists are visible but read-only from clients; static playlists are fully
  editable. It's also controllable from the command palette and the CLI (`signal
  server start|stop|status`). LAN only, no transcoding — files stream
  bit-perfect as-is — and the server keeps running across app restarts if left
  on. The server also advertises the `formPost` extension so clients like
  Symfonium send credentials in request bodies instead of URLs, songs now
  carry their genre, the `frequent` and `recent` album lists reflect real
  listening (play counts and last-played times) instead of falling back to
  newest additions, and playlists report real created/changed timestamps so
  clients only re-sync what actually changed; the settings section shows a
  paste-ready `http://<lan-ip>:port` URL instead of asking you to find your
  machine's IP. Covers are scaled server-side on request (`size` parameter,
  disk-cached in a few bucket sizes), so a phone's first sync no longer pulls
  every full-resolution artwork.

### Fixed

- Failed actions could toast `[object Object]` instead of the reason —
  backend errors arrive as structured objects and several spots stringified
  them raw. Every error toast now shows the actual message (saving a
  scrobbler token, playlist operations, artwork lookups, the new server and
  analysis controls).
- ALAC files were labeled AAC. The scanner assumed AAC for every `.m4a`; it
  now reads the MP4 sample entry to tell them apart, and a migration relabels
  already-imported files (rescans skip known paths, so scanning again would
  never have fixed them). This corrects the lossless share in health and
  stats, lets the authenticity detector analyze ALAC, and serves ALAC as
  lossless to mobile clients.
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

[Unreleased]: https://github.com/ianaya89/signal/compare/v0.1.5...HEAD
[0.1.5]: https://github.com/ianaya89/signal/compare/v0.1.3...v0.1.5
[0.1.3]: https://github.com/ianaya89/signal/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/ianaya89/signal/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/ianaya89/signal/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ianaya89/signal/releases/tag/v0.1.0
