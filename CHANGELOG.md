# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.7] - 2026-08-06

### Added

- Remote OpenSubsonic servers. Signal can now be configured with one or more
  remote Subsonic/OpenSubsonic servers — another Signal instance, Navidrome,
  Airsonic, Gonic, or anything else speaking the same protocol — browse their
  artists, albums and songs, search them, and stream a track through the same
  playback engine used for the local library, all without touching the local
  database: remote content is never scanned or cached, every browse is a live
  request and every play is a live stream. A new "remote" entry in the
  library sidebar lists each configured server's artists, albums and songs;
  settings gained a matching section to add, test and remove a server, with
  a per-server connection badge and an "allow insecure TLS" toggle for
  self-signed certificates. Playing a remote track drives the same transport
  bar, mini player, chain view and inspector as a local one, and a remote
  album auto-advances, plays gapless and honors shuffle/repeat through its
  follow-on order — remote tracks can't yet be staged into the queue
  alongside local ones, a limitation settings spells out. It's
  read-and-stream only: no catalog sync, no offline cache, and no write-back
  beyond a future scrobble to the remote server.
- Settings reorganised into tabs. Seven stacked sections in one long scroll
  became a tab strip — library, playback, appearance, scrobbling, server,
  remote, about — with the last-used tab remembered between visits. Each pane
  opens with a line stating what it is for, and per-setting explanations
  moved next to the control they describe instead of collecting at the end
  of a section. The "reset + rescan" action now says plainly that it drops
  ratings, play counts and playlists.
- Stats, doctor and logs merged into one "system" entry. Three sidebar rows
  became one tabbed pane, taking the sidebar from twelve rows to ten. Each
  tab keeps its own URL, so existing keyboard shortcuts (S for stats, L for
  logs) and command palette entries still work, along with any bookmarked
  link.
- Section headers in the library lists. Albums could already be sorted four
  ways but the result was an unbroken run of covers, so the sort was
  invisible once applied. Sorted lists now carry sticky section headers
  describing the order they are already in: initials when sorted by name,
  artist initials when sorted by artist, decades when sorted by year.
  Sorting by "recent" gets none, since a continuum has no sections. Headers
  only appear once a list is long enough to benefit — a list averaging
  fewer than two rows per section shows none at all.
- A sort for artists. The artists view had no sort and no sections: one flat
  alphabetical run, fine for a small library and unusable for a large one.
  It now sorts by name, album count or track count, with the same sort bar
  the albums view has.
- Colour as wayfinding. Each settings and system pane owns a hue, so the tab
  strip reads as a set and the pane you are in is identifiable at a glance.
  The sidebar tints the active row with the hue of the pane it opens, and
  the discover shelves — on repeat, rediscover, from your artists, never
  played — each take their own, so four peer sections stop reading as one
  stack. Connection and server status indicators glow in their own colour,
  and a live connection pulses. All of it respects `prefers-reduced-motion`.
- A primary button weight. The interface had no way to say which action a
  form was for. Submitting a new remote server, starting the server,
  installing an update and playing a remote album now carry visible weight;
  everything else stays secondary.

### Changed

- The dev server moved off port 1420. That is Tauri's default, so it
  collided with any other Tauri app in development, and because Vite runs
  with `strictPort` the second app to start simply failed to boot. Signal's
  dev server is 1421. Development only — it does not affect installed
  builds.

### Fixed

- Folder browsing was broken outright on some installs. The folders pane
  read a legacy settings key for the library root while the rest of the app
  had moved to a newer multi-root key. On any install whose roots had only
  ever been written in the newer form the key was missing, so the pane
  failed to open at all and showed an error instead of a listing. It now
  reads the same root list as every other library command, validates paths
  against all configured roots rather than one, and with several roots
  configured the top level lists the roots themselves.
- Errors displayed as `[object Object]`. Four separate copies of the same
  error-formatting helper had grown up across the interface, and all of
  them read a message field and stringified it — correct for most errors,
  but not for the ones that carry a structured payload, which rendered as
  `[object Object]` no matter which copy handled them. There is now one
  helper, it handles every error shape the backend emits, and every place
  that displays an error uses it.
- The playlists toolbar's create, "+ smart" and "import m3u…" buttons sat at
  identical weight, so nothing indicated which one the name field was for,
  and they were the only buttons in the app rendered at a larger size than
  everything else. Create now carries the primary weight and disables
  itself on an empty name.

## [0.1.6] - 2026-08-03

### Fixed

- Linux: the app crashed at startup with `mpv init failed: Null` on systems
  with a non-English locale. GTK sets the process locale from the environment
  during Tauri startup, and libmpv refuses to initialize unless `LC_NUMERIC`
  is `C`; the app now forces `LC_NUMERIC=C` before mpv initializes.

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

[Unreleased]: https://github.com/ianaya89/signal/compare/v0.1.7...HEAD
[0.1.7]: https://github.com/ianaya89/signal/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/ianaya89/signal/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/ianaya89/signal/compare/v0.1.3...v0.1.5
[0.1.3]: https://github.com/ianaya89/signal/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/ianaya89/signal/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/ianaya89/signal/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ianaya89/signal/releases/tag/v0.1.0
