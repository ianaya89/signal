# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- In-app updates. signal checks for a new release on launch and offers it in
  the status bar, the command palette (`check for updates`, `install update +
  restart`) and settings. The launch check is silent and can be turned off.
  `.deb` installs are left to apt.
- Update artifacts are signed: the macOS `.app.tar.gz` and the Linux AppImage
  each ship a minisign signature verified before anything is installed.
- Arrow key navigation in lists — `↓`/`↑` mirror `j`/`k`, `Home`/`End` mirror
  `gg`/`G`.

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
- The status bar showed a hardcoded version instead of the running one.

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

[Unreleased]: https://github.com/ianaya89/signal/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ianaya89/signal/releases/tag/v0.1.0
