# Signal

An open-source desktop Hi-Fi music player for developers, audiophiles and power users.

Not another Apple Music clone. Not another Plexamp clone. **The LazyGit of music players**: fast, keyboard-first, transparent, hackable, local-first. No cloud, no account, no telemetry, no Electron.

## Stack

- **Backend**: Rust stable, Tauri v2
- **Playback**: libmpv (FLAC, ALAC, WAV, AIFF, AAC, MP3, OGG, Opus — gapless, ReplayGain, exclusive mode, auto sample-rate switching)
- **Data**: SQLite + sqlx, FTS5 search, lofty metadata, notify fs-watcher
- **Frontend**: React, TypeScript, TailwindCSS, shadcn/ui, TanStack Router/Query, Zustand, uPlot

## Design docs

| Doc | Covers |
|-----|--------|
| [01 — Architecture](docs/01-architecture.md) | Components, threads, event bus, data flows, error strategy |
| [02 — Workspace](docs/02-workspace.md) | Cargo workspace layout, crates, dependency rules, feature flags |
| [03 — Database schema](docs/03-database-schema.md) | Full DDL, FTS5 setup, smart playlist rules, repositories |
| [04 — Player (libmpv)](docs/04-player-libmpv.md) | mpv wrapper, gapless, ReplayGain, exclusive mode, bit-perfect detection |
| [05 — IPC API](docs/05-ipc-api.md) | Tauri command catalog, events, DTOs, artwork protocol |
| [06 — Frontend](docs/06-frontend.md) | Routing, state split (Query vs Zustand), components, event bridge |
| [07 — Roadmap](docs/07-roadmap.md) | Milestones M0–M5, exit criteria, risk register |
| [08 — Plugins](docs/08-plugins.md) | Plugin trait, tiers, lifecycle, MPRIS / Last.fm / Discord sketches |
| [09 — Keyboard](docs/09-keyboard.md) | Mode stack, full binding tables, rebinding, discoverability |
| [10 — Design system](docs/10-design-system.md) | Palette, typography, density, component specs, motion rules |

## Development

Rust toolchain is managed by [mise](https://mise.jdx.dev) (`mise.toml`).

```sh
# first time
mise trust && mise install
pnpm install
brew install pkgconf mpv   # libmpv + its pkg-config metadata

# run
pnpm tauri dev
```

If `cargo` is not found, either activate mise in your shell
(`eval "$(mise activate zsh)"` in `~/.zshrc`) or run `mise exec -- pnpm tauri dev`.

## Principles

- Everything is local. Everything works offline.
- Every interaction feels instant. Performance is a feature.
- Every action has a keyboard shortcut. Mouse optional.
- Expose technical details instead of hiding them.
