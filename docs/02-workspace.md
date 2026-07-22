# Signal Cargo Workspace

Signal's backend is a Cargo workspace of six domain crates plus the `src-tauri` application shell, with a strict one-directional dependency graph rooted at `signal-core`. This document lays out the directory tree, the root manifest, each crate's responsibility and public API, the dependency rules, feature flags, testing layout, and the day-to-day build commands.

## Directory tree

```
rocola/                                # repo root
├── Cargo.toml                         # [workspace], workspace.dependencies, workspace.lints
├── Cargo.lock
├── rust-toolchain.toml                # pinned stable channel
├── tauri.conf.json                    # -> src-tauri/tauri.conf.json (symlink not used; kept in src-tauri)
├── migrations/                        # sqlx migrations, shared by signal-db
│   ├── 0001_init.sql
│   ├── 0002_fts5_tracks.sql
│   ├── 0003_smart_playlists.sql
│   └── ...
├── fixtures/                          # tiny real audio files used by tests/benches
│   ├── flac/short-44100-16.flac
│   ├── mp3/short-cbr-320.mp3
│   ├── ogg/short-opus.ogg
│   ├── wav/short-48000-24.wav
│   └── tags/malformed-id3.mp3
├── crates/
│   ├── signal-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                 # re-exports of the module tree below
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── track.rs           # Track, TrackTechnical
│   │       │   ├── album.rs           # Album
│   │       │   ├── artist.rs          # Artist
│   │       │   ├── genre.rs           # Genre
│   │       │   ├── playlist.rs        # Playlist, SmartPlaylist, SmartRule
│   │       │   ├── queue.rs           # QueueItem
│   │       │   ├── stats.rs           # PlayEvent
│   │       │   └── device.rs          # AudioDevice, PlayerState
│   │       ├── events.rs              # SignalEvent, EventBus
│   │       ├── error.rs               # SignalError
│   │       └── config.rs              # AppConfig, paths (data dir, cache dir)
│   │
│   ├── signal-db/
│   │   ├── Cargo.toml
│   │   ├── tests/
│   │   │   └── repositories.rs        # integration tests against a temp sqlite file
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pool.rs                # init(), connection options, WAL setup
│   │       ├── migrate.rs             # sqlx::migrate! wiring
│   │       └── repositories/
│   │           ├── mod.rs
│   │           ├── tracks.rs
│   │           ├── albums.rs
│   │           ├── artists.rs
│   │           ├── playlists.rs
│   │           ├── smart_playlists.rs
│   │           ├── queue.rs
│   │           ├── stats.rs
│   │           └── settings.rs
│   │
│   ├── signal-scanner/
│   │   ├── Cargo.toml
│   │   ├── benches/
│   │   │   └── scan_throughput.rs     # criterion: files/sec over fixtures/
│   │   ├── tests/
│   │   │   └── incremental_scan.rs
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scanner.rs             # Scanner, ScanJob, orchestration
│   │       ├── tags.rs                # lofty extraction -> TrackTechnical
│   │       ├── artwork.rs             # embedded + folder artwork resolution
│   │       ├── watcher.rs             # notify setup, debouncing
│   │       └── diff.rs                # incremental add/update/remove detection
│   │
│   ├── signal-player/
│   │   ├── Cargo.toml
│   │   ├── tests/
│   │   │   └── gapless.rs             # requires a real/mocked mpv handle
│   │   └── src/
│   │       ├── lib.rs                 # Player public API
│   │       ├── mpv.rs                 # libmpv2 handle wrapper, command dispatch
│   │       ├── event_loop.rs          # mpv event thread + bridging to tokio
│   │       ├── gapless.rs             # queue lookahead, crossfade-free gapless
│   │       ├── replaygain.rs          # RG mode application (track/album/off)
│   │       ├── device.rs              # AudioDevice enumeration, selection
│   │       └── exclusive.rs           # per-OS exclusive/hog-mode handling
│   │
│   ├── signal-search/
│   │   ├── Cargo.toml
│   │   ├── benches/
│   │   │   └── query_compile.rs       # criterion: parse+compile latency
│   │   ├── tests/
│   │   │   └── query_language.rs
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── lexer.rs
│   │       ├── parser.rs              # tokens -> AST
│   │       ├── ast.rs
│   │       ├── schema.rs              # valid fields, types, operators
│   │       └── compile.rs             # AST -> parameterized SQL/FTS5
│   │
│   └── signal-plugins/
│       ├── Cargo.toml
│       ├── tests/
│       │   └── lifecycle.rs
│       └── src/
│           ├── lib.rs
│           ├── plugin.rs              # Plugin trait, PluginContext
│           ├── registry.rs            # PluginHost, load/enable/disable
│           ├── lastfm.rs
│           ├── listenbrainz.rs
│           ├── lyrics.rs
│           ├── mpris.rs               # Linux only
│           ├── discord_rp.rs
│           └── home_assistant.rs
│
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   └── src/
│       ├── main.rs
│       ├── state.rs                   # AppState definition + setup()
│       ├── bridge.rs                  # EventBus -> AppHandle::emit task
│       └── commands/
│           ├── mod.rs
│           ├── player.rs              # player_play, player_pause, ...
│           ├── queue.rs               # queue_add, queue_remove, ...
│           ├── library.rs             # library_scan, library_list_albums, ...
│           ├── search.rs              # search_query
│           ├── palette.rs             # palette_execute
│           ├── device.rs              # device_list, device_select
│           ├── stats.rs               # stats_overview
│           ├── logs.rs                # logs_tail
│           ├── playlist.rs            # playlist_create, playlist_add_tracks
│           └── settings.rs            # settings_get, settings_set
│
├── src/                                # React frontend (Vite root)
│   ├── main.tsx
│   ├── routes/                         # TanStack Router route tree
│   ├── stores/                         # Zustand: usePlayerStore, useQueueStore, ...
│   ├── ipc/                            # invoke()/listen() typed wrappers
│   ├── components/
│   ├── hooks/
│   └── lib/
│
└── docs/
    ├── 01-architecture.md
    └── 02-workspace.md
```

## Root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/signal-core",
    "crates/signal-db",
    "crates/signal-scanner",
    "crates/signal-player",
    "crates/signal-search",
    "crates/signal-plugins",
    "src-tauri",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.82"
license = "GPL-3.0-or-later"
repository = "https://github.com/signal-audio/signal"

[workspace.dependencies]
signal-core = { path = "crates/signal-core" }
signal-db = { path = "crates/signal-db" }
signal-scanner = { path = "crates/signal-scanner" }
signal-player = { path = "crates/signal-player" }
signal-search = { path = "crates/signal-search" }
signal-plugins = { path = "crates/signal-plugins" }

tauri = { version = "2" }   # macos-private-api requires app.macOSPrivateApi in tauri.conf.json; enable together when needed
tauri-build = "2"

tokio = { version = "1.41", features = ["rt-multi-thread", "macros", "sync", "time", "fs"] }

sqlx = { version = "0.8", default-features = false, features = [
    "runtime-tokio",
    "sqlite",
    "macros",
    "migrate",
    "chrono",
] }

lofty = "0.21"
notify = "7.0"
notify-debouncer-full = "0.4"

libmpv2 = "3.0"

serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"

tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.11", features = ["v4", "serde"] }
walkdir = "2.5"

criterion = "0.5"
tempfile = "3.13"

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "warn"
```

`signal-player` is the sole exception to `unsafe_code = "deny"`, since the libmpv2 FFI boundary occasionally requires raw pointer handling not fully covered by the safe wrapper (custom render contexts, property observation callbacks). It overrides the workspace lint locally:

```toml
# crates/signal-player/Cargo.toml
# Cargo forbids combining `workspace = true` with local overrides in [lints],
# so this crate mirrors the workspace set locally with unsafe_code relaxed:
[lints.rust]
unsafe_code = "warn"   # allowed, but must be justified per call site with a comment

[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "warn"
```

Every other crate, including `src-tauri`, inherits `unsafe_code = "deny"` unmodified — if a future dependency needs `unsafe`, it needs to live behind the `signal-player` boundary, not leak into the app shell.

## Per-crate responsibilities and public API

### `signal-core`
Holds every domain type shared across the workspace — `Track`, `Album`, `Artist`, `Genre`, `Playlist`, `SmartPlaylist`, `QueueItem`, `PlayEvent`, `AudioDevice`, `PlayerState`, `TrackTechnical` — plus the top-level `SignalError`, the `SignalEvent`/`EventBus` pair, and app-wide configuration/path resolution. It has zero internal dependencies and exists purely so every other crate can speak the same vocabulary without depending on each other.

```rust
pub struct Track { /* id, title, album_id, artist_id, technical: TrackTechnical, ... */ }
pub struct TrackTechnical { /* codec, bitrate_kbps, bit_depth, sample_rate_hz, ... */ }
pub enum SignalError { Db(String), Player(String), Scanner(String), Search(String), Plugin(String), InvalidQuery { reason: String }, Io(String) }
pub enum SignalEvent { PlayerStateChanged(PlayerState), ScannerDone { .. }, QueueChanged(Vec<QueueItem>), LogLine { .. }, /* ... */ }
pub struct EventBus { /* broadcast wrapper */ }
pub struct AppConfig { pub data_dir: PathBuf, pub cache_dir: PathBuf, pub db_path: PathBuf }
```

### `signal-db`
Owns the sqlx `SqlitePool`, runs migrations, and exposes repository structs that are the only sanctioned path to the database. No other crate builds SQL directly against the pool.

```rust
pub async fn init(config: &AppConfig) -> Result<DbPool, DbError>;
pub struct DbPool(SqlitePool);
pub struct TrackRepo<'a> { /* ... */ }
impl<'a> TrackRepo<'a> {
    pub async fn upsert_batch(&self, tracks: &[Track]) -> Result<(), DbError>;
    pub async fn get(&self, id: i64) -> Result<Track, DbError>;
    pub async fn list_by_album(&self, album_id: i64) -> Result<Vec<Track>, DbError>;
}
pub struct QueueRepo<'a> { /* persist/restore queue_items */ }
pub struct StatsRepo<'a> { /* flush PlayEvent batches, aggregate for stats_overview */ }
```

### `signal-scanner`
Walks configured library roots, extracts metadata via `lofty`, resolves artwork, watches for filesystem changes via `notify`, and turns both full and incremental scans into batched writes through `signal-db`. Depends on `signal-core` and `signal-db`.

```rust
pub struct Scanner { /* ... */ }
impl Scanner {
    pub fn new(db: Arc<DbPool>, events: EventBus, roots: Vec<PathBuf>) -> Self;
    pub async fn scan_full(&self) -> Result<ScanReport, ScannerError>;
    pub async fn scan_incremental(&self, paths: &[PathBuf]) -> Result<ScanReport, ScannerError>;
    pub fn watch(&self) -> Result<(), ScannerError>; // spawns notify watchers
}
pub struct ScanReport { pub added: u32, pub updated: u32, pub removed: u32 }
```

### `signal-player`
Wraps libmpv2 behind a safe, async-friendly API: loading tracks, gapless queueing, ReplayGain application, device enumeration/selection, and exclusive-mode output. Depends only on `signal-core`.

```rust
pub struct Player { /* ... */ }
impl Player {
    pub async fn new(config: PlayerConfig, events: EventBus) -> Result<Self, PlayerError>;
    pub async fn load(&self, track: &Track) -> Result<(), PlayerError>;
    pub async fn play(&self) -> Result<(), PlayerError>;
    pub async fn pause(&self) -> Result<(), PlayerError>;
    pub async fn seek(&self, position_ms: u64) -> Result<(), PlayerError>;
    pub async fn set_volume(&self, volume: f32) -> Result<(), PlayerError>;
    pub async fn queue_next(&self, track: &Track) -> Result<(), PlayerError>; // gapless lookahead
    pub fn state(&self) -> PlayerState;
    pub async fn list_devices(&self) -> Result<Vec<AudioDevice>, PlayerError>;
    pub async fn select_device(&self, device_id: &str) -> Result<(), PlayerError>;
}
```

### `signal-search`
Parses the query language (`artist:cerati`, `year:1999`, `rating>4`, `codec:flac`, `added:last-week`, ...) and compiles it into parameterized SQL that mixes FTS5 `MATCH` clauses with structured `WHERE` predicates, executed through `signal-db`. Depends on `signal-core` and `signal-db`.

```rust
pub fn parse(query: &str) -> Result<QueryAst, SearchError>;
pub fn compile(ast: &QueryAst) -> CompiledQuery; // -> sql string + bound params
pub async fn search(db: &DbPool, query: &str, limit: u32, offset: u32) -> Result<Vec<Track>, SearchError>;
pub static SCHEMA: &[FieldDef]; // field name -> type + allowed operators, used by parser and by editor autocomplete
```

### `signal-plugins`
Defines the plugin trait and hosts the built-in plugins (Last.fm, ListenBrainz, lyrics, MPRIS, Discord RP, Home Assistant), subscribing to the event bus and calling into `signal-player`/`signal-db` as needed for their own read-only needs (e.g. MPRIS needs current `PlayerState` to answer D-Bus queries). Depends on `signal-core`, `signal-player`, `signal-db`.

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    fn id(&self) -> &'static str;
    async fn start(&mut self, ctx: PluginContext) -> Result<(), PluginError>;
    async fn stop(&mut self) -> Result<(), PluginError>;
}
pub struct PluginContext { pub events: EventBus, pub player: Arc<Player>, pub db: Arc<DbPool> }
pub struct PluginHost { /* ... */ }
impl PluginHost {
    pub fn new(events: EventBus) -> Self;
    pub async fn load_enabled(&mut self, settings: &PluginSettings) -> Result<(), PluginError>;
    pub async fn enable(&mut self, id: &str) -> Result<(), PluginError>;
    pub async fn disable(&mut self, id: &str) -> Result<(), PluginError>;
}
```

### `src-tauri`
The only crate that depends on all six domain crates. Owns `AppState`, wires every IPC command handler, and runs the single task that bridges `EventBus` broadcasts to Tauri's frontend event system. Contains no domain logic of its own beyond argument marshalling — a command handler's body is a few lines that call into the appropriate crate and map its `Result` into `Result<T, SignalError>`.

## Dependency direction

```
                     signal-core
                    /  |   |   \  \
                   /   |   |    \  \
        signal-db  signal-player  |  \
             \       /            |   \
        signal-scanner       signal-search
                   \             /
                    \           /
                signal-plugins (also -> signal-player, signal-db)
                          \
                           \
                        src-tauri  (depends on ALL of the above)
```

Rules, enforced by `cargo` (a cycle simply won't compile) and checked in review:

- `signal-core` depends on no workspace crate. It may depend on external crates only (`serde`, `thiserror`, `tokio` for the broadcast type).
- Every other crate depends on `signal-core`.
- `signal-db` depends only on `signal-core`.
- `signal-player` depends only on `signal-core`.
- `signal-scanner` depends on `signal-core` and `signal-db` (it writes scan results through repositories).
- `signal-search` depends on `signal-core` and `signal-db` (it executes compiled queries through the pool).
- `signal-plugins` depends on `signal-core`, `signal-db`, and `signal-player` (plugins read state and call playback control, e.g. MPRIS `Next`/`Previous`).
- No domain crate ever depends on `src-tauri`. `src-tauri` is a leaf consumer only — this is what keeps every domain crate independently testable and, longer term, reusable outside the Tauri shell (e.g. a future headless/CLI mode).
- `signal-scanner` and `signal-search` never depend on each other, and neither depends on `signal-player` — scanning and searching have no playback concerns.

## Feature flags

```toml
# crates/signal-player/Cargo.toml
[features]
default = ["exclusive-mode"]
exclusive-mode = []          # gates device.rs hog-mode paths; implementation is
                              # per-OS (WASAPI exclusive on Windows, AudioHardware
                              # hog mode on macOS, ALSA hw: device on Linux) behind
                              # #[cfg(target_os = "...")] inside exclusive.rs

# crates/signal-plugins/Cargo.toml
[features]
default = ["lastfm", "listenbrainz", "lyrics", "discord-rp", "home-assistant"]
lastfm = ["dep:reqwest"]
listenbrainz = ["dep:reqwest"]
lyrics = ["dep:reqwest"]
discord-rp = ["dep:discord-rich-presence"]
home-assistant = ["dep:reqwest"]
mpris = ["dep:zbus"]         # only meaningful on Linux; compiled out elsewhere

[target.'cfg(target_os = "linux")'.dependencies]
zbus = { version = "4.4", optional = true }

# crates/signal-db/Cargo.toml
[features]
default = ["bundled-sqlite"]
bundled-sqlite = ["sqlx/sqlite"]    # link SQLite statically via libsqlite3-sys bundled feature
system-sqlite = []                   # opt-out for Linux distro packages that require dynamic linking
```

`src-tauri/Cargo.toml` enables the OS-appropriate feature set explicitly per platform target rather than relying on crate-level `default` alone, so a Linux build gets `signal-plugins/mpris` while macOS/Windows builds don't pull in `zbus` at all:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
signal-plugins = { workspace = true, features = ["mpris"] }

[target.'cfg(not(target_os = "linux"))'.dependencies]
signal-plugins = { workspace = true }
```

## Testing layout

- **Unit tests** live in-crate, in `#[cfg(test)] mod tests` blocks next to the code they cover (e.g. `signal-search/src/parser.rs` has its own tokenizer/AST tests inline).
- **Integration tests** live in each crate's `tests/` directory and exercise the crate's public API end-to-end: `signal-db/tests/repositories.rs` runs real migrations against a `tempfile`-backed SQLite database; `signal-scanner/tests/incremental_scan.rs` points a `Scanner` at `fixtures/` and asserts on the resulting `ScanReport` and DB rows.
- **Benchmarks** use `criterion` and live in `benches/` in the two crates where performance is a first-class concern: `signal-search/benches/query_compile.rs` (parse+compile latency, must stay well under the "instant" search budget) and `signal-scanner/benches/scan_throughput.rs` (files/sec against the `fixtures/` corpus, tracked over time to catch regressions from lofty upgrades or added metadata extraction).
- **`fixtures/`** at the repo root holds small (a few seconds, kept under a few hundred KB each) real audio files covering the supported format matrix (FLAC, ALAC container, WAV, AIFF, AAC, MP3, OGG, Opus) plus a few deliberately malformed files (bad ID3 frame, truncated FLAC header) to exercise the scanner's error-tolerance path. These are checked into the repo since they're tiny and tests must be runnable offline with no network fetch.
- `signal-player` integration tests that require an actual mpv/audio backend are marked `#[ignore]` by default and run explicitly in CI on each target OS with a real (often virtual/null) audio device, since spinning up audio hardware access isn't appropriate for a default `cargo test` run on a contributor's machine.

## Build commands

```sh
# Local development: hot-reloads both the Rust backend and the Vite frontend
cargo tauri dev

# Production release build (creates platform installers under src-tauri/target/release/bundle)
cargo tauri build

# Run all workspace tests (unit + integration; excludes #[ignore]'d hardware tests)
cargo test --workspace

# Run a single crate's tests
cargo test -p signal-search

# Run the ignored hardware-dependent player tests explicitly (CI, or locally with a device)
cargo test -p signal-player -- --ignored

# Benchmarks (criterion; results under target/criterion/)
cargo bench -p signal-search
cargo bench -p signal-scanner

# Lint: zero warnings tolerated, matches CI gate
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format check (CI) / format in place (local)
cargo fmt --all -- --check
cargo fmt --all
```

CI runs `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` on every push, plus `cargo tauri build` on release tags for each of the three target platforms.
