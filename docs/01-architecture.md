# Signal Architecture

Signal is a Tauri v2 desktop app split into a Rust backend (six focused crates plus the `src-tauri` shell) and a React frontend. The backend owns all state and does all I/O; the frontend is a thin, event-driven view over it. This document covers the component layout, the process/thread model, the event bus, three end-to-end data flows, error handling, state ownership, and startup/shutdown.

## High-level component diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│  Frontend (React + TS)                                                │
│  TanStack Router/Query, Zustand stores, shadcn/ui, uPlot               │
│                                                                        │
│   invoke("player_play", …) ──────────┐      ┌── listen("player:state") │
└───────────────────────────────────────┼──────┼─────────────────────────┘
                                        │      │
                              Tauri IPC │      │ Tauri events
                                        ▼      │
┌──────────────────────────────────────────────────────────────────────┐
│  src-tauri (Tauri v2 shell)                                           │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │ AppState (Arc handles, no global mutable state)                 │  │
│  │   db: Arc<DbPool>        player: Arc<Player>                    │  │
│  │   scanner: Arc<Scanner>  plugins: Arc<PluginHost>                │  │
│  │   events: signal_core::EventBus (tokio broadcast)                │  │
│  └────────────────────────────────────────────────────────────────┘  │
│  IPC command handlers (player_*, queue_*, library_*, search_*, …)     │
│  Event bridge task: EventBus rx -> tauri AppHandle::emit              │
└───────┬───────────────┬───────────────┬───────────────┬──────────────┘
        │                │               │               │
        ▼                ▼               ▼               ▼
┌───────────────┐ ┌─────────────┐ ┌──────────────┐ ┌────────────────┐
│ signal-db      │ │ signal-player│ │ signal-scanner│ │ signal-search   │
│ sqlx pool,     │ │ libmpv       │ │ lofty + notify│ │ query parser -> │
│ migrations,    │ │ wrapper:     │ │ incremental   │ │ SQL / FTS5      │
│ repositories   │ │ gapless, RG, │ │ scan pipeline │ │                 │
│                │ │ devices,     │ │               │ │                 │
│                │ │ exclusive    │ │               │ │                 │
└───────┬────────┘ └──────┬───────┘ └──────┬────────┘ └────────┬────────┘
        │                 │                │                    │
        ▼                 ▼                ▼                    │
┌───────────────┐ ┌─────────────┐ ┌──────────────┐              │
│ SQLite (WAL)   │ │ libmpv /    │ │ Filesystem    │              │
│ tracks_fts     │ │ OS audio    │ │ (watched dirs)│◄─────────────┘
│ (FTS5)         │ │ backend     │ │               │  (search reads same DB)
└───────────────┘ └─────────────┘ └──────────────┘

        signal-core (domain types, SignalError, SignalEvent, config)
        — depended on by every crate above, depends on none of them —

        signal-plugins (Last.fm, ListenBrainz, lyrics, MPRIS, Discord RP, HA)
        — subscribes to the event bus, calls out to signal-db/signal-player —
```

Every arrow into SQLite, libmpv, or the filesystem is mediated by exactly one crate. Nothing outside `signal-db` touches the database connection pool directly, nothing outside `signal-player` calls into libmpv, and nothing outside `signal-scanner` reads directory trees for library purposes.

## Process/thread model

Signal is a single OS process. Concurrency inside it is organized as:

```
Main thread (OS-owned, required by Tauri/webview)
  └─ runs the Tauri event loop, owns the WebView, dispatches IPC calls
     into the tokio runtime via tauri::async_runtime

Tokio multi-threaded runtime (started in main(), lives for process lifetime)
  ├─ IPC command futures (short-lived, one per invoke())
  ├─ EventBus bridge task (long-lived: broadcast::Receiver<SignalEvent> -> emit)
  ├─ signal-db pool tasks (sqlx's own async connection tasks)
  ├─ signal-search query tasks
  └─ signal-plugins tasks (one supervised task per active plugin)

mpv event thread (spawned by signal-player, NOT a tokio task)
  └─ blocking loop on mpv_wait_event(); libmpv's C API is not async, so this
     runs on its own std::thread and forwards mpv events into a tokio
     mpsc channel that a small async task turns into SignalEvent::Player*

Scanner worker pool (tokio blocking pool via spawn_blocking, sized to
  num_cpus, bounded)
  └─ lofty tag reads are blocking/CPU-bound; each file read is dispatched
     via spawn_blocking so it doesn't starve the async runtime

notify watcher thread (spawned by the `notify` crate itself)
  └─ OS-level fs events (FSEvents/inotify/ReadDirectoryChangesW) arrive on
     a std::sync::mpsc channel; a bridge task forwards debounced events
     into the scanner's incremental-update queue
```

Communication rules:

- Cross-thread, non-async boundaries (mpv callbacks, notify callbacks) use `std::sync::mpsc` or `crossbeam-channel`, never raw shared mutable state.
- Cross-task, async-to-async communication uses `tokio::sync::mpsc` for point-to-point work queues (e.g. scanner job queue) and `tokio::sync::broadcast` for one-to-many notification (the event bus).
- No thread ever locks a `Mutex` across an `.await` point; anything shared across async boundaries is either behind a `tokio::sync::RwLock` (read-heavy, e.g. current `PlayerState` snapshot) or owned by a single task that others message.

## The event bus

`signal-core` defines the bus payload and a thin wrapper around `tokio::sync::broadcast`:

```rust
// signal-core/src/events.rs

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum SignalEvent {
    PlayerStateChanged(PlayerState),
    PlayerProgress { position_ms: u64, duration_ms: u64 },
    TrackChanged(Track),
    DeviceChanged(AudioDevice),

    ScannerProgress { scanned: u32, total: u32, current_path: String },
    ScannerDone { added: u32, updated: u32, removed: u32 },

    QueueChanged(Vec<QueueItem>),

    LogLine { level: LogLevel, target: String, message: String },

    PluginError { plugin: String, message: String },
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<SignalEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: SignalEvent) {
        // Send errors (no receivers) are expected and ignored; the bus
        // has no guaranteed subscriber at startup.
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SignalEvent> {
        self.tx.subscribe()
    }
}
```

**Publishers**: `signal-player` (state/progress/track-changed/device-changed), `signal-scanner` (scanner progress/done, which also indirectly triggers library refetches), `signal-db` (queue changed, on any queue mutation), the `tracing` subscriber layer in `src-tauri` (log lines, see below), and `signal-plugins` (plugin errors).

**Subscribers**:
1. The **Tauri event bridge task**, spawned once in `src-tauri::setup()`, holds a subscription and forwards every `SignalEvent` to the frontend via `AppHandle::emit(name, payload)`, where `name` is the corresponding `"player:state"`, `"scanner:progress"`, etc. string from the shared naming convention. This is the only place that translates internal enum variants into named Tauri events.
2. `signal-plugins`' `PluginHost` holds a subscription so plugins (Discord RP, Last.fm scrobbling, Home Assistant) react to `TrackChanged`/`PlayerStateChanged` without the player crate knowing plugins exist.
3. The in-app log viewer's backend support (`logs_tail` command) keeps a ring buffer fed by `LogLine` events so a freshly opened log pane has recent history, not just events from the moment it subscribed.

Log lines reach the bus through a custom `tracing_subscriber::Layer` installed in `src-tauri::main()` that converts each `tracing` event into `SignalEvent::LogLine` and publishes it — so `tracing::warn!(target: "signal_scanner", "skipped corrupt file")` shows up in the in-app log viewer without any crate needing to know about the event bus directly.

The bus is intentionally one-directional (backend -> frontend/plugins). Frontend -> backend communication always goes through IPC commands, never through a symmetric event channel; this keeps the mutation path (commands, request/response, easy to trace and test) distinct from the notification path (events, fire-and-forget, easy to fan out).

## Data flow walkthroughs

### (a) User hits play on a track

1. User selects a track row and presses `Enter` (or clicks Play); the React key handler calls `invoke("player_play", { trackId })`.
2. Tauri routes the IPC call to the `player_play` command handler in `src-tauri`, which pulls `state.player: Arc<Player>` and `state.db: Arc<DbPool>` out of `AppState`.
3. The handler asks `signal-db` for the full `Track` + `TrackTechnical` row (file path, codec, sample rate, ReplayGain values) via a repository call.
4. The handler calls `player.load(track).await` on `signal-player`. Internally this issues `mpv_command_async(["loadfile", path])` over the libmpv handle, sets ReplayGain-derived `af`/volume options if RG is enabled, and — if automatic sample-rate switching is on — checks the track's `sample_rate_hz` against the current device's active rate and reopens the audio device at the new rate if they differ.
5. libmpv performs the actual demux/decode/output setup on its own internal threads; the mpv event thread in `signal-player` blocks on `mpv_wait_event()` and receives a `MPV_EVENT_FILE_LOADED` followed by `MPV_EVENT_PLAYBACK_RESTART`.
6. The mpv event thread forwards these as an internal message over its `mpsc` channel; the small async task on the other end updates the shared `RwLock<PlayerState>` and calls `event_bus.publish(SignalEvent::PlayerStateChanged(state))` and `SignalEvent::TrackChanged(track)`.
7. The Tauri event bridge task receives both from its `broadcast::Receiver` and emits `"player:state"` and `"player:track-changed"` to the webview.
8. The frontend's Zustand player store, subscribed via `listen("player:state", …)` and `listen("player:track-changed", …)` in a root-level effect, updates in place; the now-playing bar and Inspector pane (codec/bit depth/sample rate/DR/bit-perfect flag) re-render from the new store slice — no additional IPC round trip needed for the UI to reflect playback.
9. Once playback is confirmed running, `signal-player` also emits periodic `SignalEvent::PlayerProgress` (driven by mpv's `time-pos` property observation, throttled to ~4 Hz) which the bridge forwards as `"player:progress"` for the seek bar.
10. On track completion, `signal-db` records a `PlayEvent` (used later for listening statistics) once playback exceeds the configured "counted as played" threshold.

### (b) Scanner finds a new album while the app is running

1. The `notify` watcher thread, set up during startup for every configured library root, receives a raw OS filesystem event (e.g. `IN_CLOSE_WRITE` on Linux) when new files finish being written into a watched folder.
2. The watcher's callback pushes the raw event onto a `std::sync::mpsc` channel; a small bridge task in `signal-scanner` reads from it, debounces bursts (a whole album copy generates many events) over a short window (~500 ms of quiet), and coalesces them into a set of changed paths.
3. The bridge task enqueues an incremental scan job — the set of changed paths, not a full rescan — onto the scanner's internal `tokio::sync::mpsc` work queue.
4. The scanner worker pool picks up the job; each new/changed file is dispatched via `spawn_blocking` to run `lofty::Probe::open(path).read()`, extracting tags, embedded artwork, and technical metadata (codec, bit depth, sample rate, channels, encoder) into a `TrackTechnical`.
5. As files complete, the scanner batches them and calls `signal-db` repository methods to upsert `artists`/`albums`/`tracks`/`track_genres` rows in a single transaction per batch, and to update `tracks_fts` (FTS5 external-content table) via the matching `INSERT`/`UPDATE` triggers or explicit `INSERT INTO tracks_fts(rowid, ...)` maintenance statements.
6. Folder-level artwork (`cover.jpg`, `folder.png`) is resolved once per album directory and cached to the app's data dir; the album row's `artwork_path` is updated if it changed.
7. Throughout the batch, the scanner publishes `SignalEvent::ScannerProgress { scanned, total, current_path }` at a throttled rate; the bridge forwards `"scanner:progress"` so the frontend can show a subtle progress indicator (not a blocking modal — scanning must never block playback or browsing).
8. When the job completes, the scanner publishes `SignalEvent::ScannerDone { added, updated, removed }`, forwarded as `"scanner:done"`.
9. The frontend's library views, backed by TanStack Query, treat `"scanner:done"` as a cache-invalidation signal: `queryClient.invalidateQueries({ queryKey: ["albums"] })` etc., so the new album appears without a manual refresh, and only after the DB write is actually committed (avoiding flicker on partial data).
10. If a file fails to parse (corrupt tag, unsupported container), the scanner logs a `tracing::warn!` (which flows to the log viewer per the event bus section above) and continues the batch — one bad file never aborts a scan.

### (c) User types a search query

1. User types `artist:cerati year:1999` into the command-palette-adjacent search box; the frontend debounces keystrokes (~150 ms) and calls `invoke("search_query", { query, limit, offset })`.
2. The `search_query` IPC handler in `src-tauri` hands the raw string to `signal-search`.
3. `signal-search`'s parser tokenizes the query language (bare terms, `field:value`, `field>value`/`field<value` for numeric/date fields, `added:last-week` relative-date sugar) into an AST, validating field names against a fixed schema table (`artist`, `year`, `rating`, `codec`, `sampleRate`, `duration`, `added`, …).
4. The AST is compiled to a parameterized SQL statement: bare terms and any full-text-eligible fields become an FTS5 `MATCH` clause against `tracks_fts`, joined back to `tracks`/`albums`/`artists`; structured comparisons (`year=1999`, `rating>4`, `sampleRate>48000`) become ordinary `WHERE` predicates ANDed together — FTS5 handles relevance-ranked text matching, plain SQL handles exact/range filtering, and the compiler picks whichever (or both) the query needs.
5. `signal-search` executes the compiled query through `signal-db`'s pool (read-only connection, `sqlx::query_as` into `Track`/`Album` view structs), respecting `limit`/`offset` for pagination.
6. Results return synchronously as the IPC response (search is request/response, not event-based — there's no reason to fan this out on the bus since only the requesting view cares).
7. The frontend receives the array of matches and renders it via a TanStack Query cache keyed on the raw query string, so re-typing an already-seen query is instant from cache.
8. If the parser rejects the query (unknown field, malformed comparison), `signal-search` returns a `SignalError::InvalidQuery { reason }` which the IPC layer serializes as an `Err` variant; the frontend shows inline validation text under the search box rather than a toast, since this is a direct, expected consequence of user input rather than a system failure.
9. Because FTS5 queries against SQLite are typically sub-millisecond to low-millisecond on libraries in the tens-of-thousands-of-tracks range, no loading spinner is shown for typical queries — consistent with the "everything instant" product principle; a spinner only appears if a query is still pending after ~120 ms.

## Error handling strategy

Each crate defines its own error enum with `thiserror`, scoped to what can actually go wrong in that crate:

```rust
// signal-db/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database connection failed: {0}")]
    Connection(#[from] sqlx::Error),
    #[error("migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("record not found: {kind} {id}")]
    NotFound { kind: &'static str, id: i64 },
}

// signal-player/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("mpv command failed: {0}")]
    Mpv(#[from] libmpv2::Error),
    #[error("unsupported codec: {0}")]
    UnsupportedCodec(String),
    #[error("no output device available")]
    NoDevice,
}
```

`signal-core` defines a top-level `SignalError` that wraps every crate error via `#[from]`, plus its own cross-cutting variants, and derives `serde::Serialize` so it can cross the Tauri IPC boundary directly as a command's `Err` type:

```rust
// signal-core/src/error.rs
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum SignalError {
    #[error("database error: {0}")]
    Db(String),
    #[error("player error: {0}")]
    Player(String),
    #[error("scanner error: {0}")]
    Scanner(String),
    #[error("search error: {0}")]
    Search(String),
    #[error("plugin error: {0}")]
    Plugin(String),
    #[error("invalid query: {reason}")]
    InvalidQuery { reason: String },
    #[error("io error: {0}")]
    Io(String),
}

impl From<signal_db::DbError> for SignalError {
    fn from(e: signal_db::DbError) -> Self {
        SignalError::Db(e.to_string())
    }
}
// ...one From impl per crate error, converting to an owned String since
// the underlying error types (e.g. sqlx::Error) are not themselves
// Serialize.
```

Every Tauri command returns `Result<T, SignalError>`; Tauri's IPC layer serializes the `Err` case to the frontend automatically as a rejected promise. The frontend's IPC wrapper (a thin `invoke<T>()` helper) catches these rejections in one place and:

- Shows a **toast** (shadcn `sonner`/`toast`) with the human-readable `SignalError` message, for anything the user directly triggered (failed scan start, playback failure, device unavailable).
- Always also logs the same error as a `SignalEvent::LogLine` at `ERROR` level (emitted from the command handler before returning, not duplicated by the frontend) so the in-app log viewer has a complete record even for errors the user dismissed as a toast — the toast is ephemeral UX, the log line is the durable trail.

Errors that originate outside a direct user action (e.g. the fs watcher losing access to a network-mounted library folder, a plugin's HTTP call failing) never produce a toast — there's no command awaiting them — and instead only flow to the log viewer via `SignalEvent::LogLine`/`SignalEvent::PluginError`, keeping toasts reserved for things the user did and is waiting on.

`unwrap()`/`expect()` are disallowed outside tests and truly-unreachable invariants (enforced via clippy lints, see `02-workspace.md`); every fallible operation returns a typed `Result`.

## Where state lives

- **SQLite is the single source of truth for the library.** Artists, albums, tracks, genres, playlists, smart playlist rules, and play history all live there. No in-memory cache of the library is kept in `signal-db` beyond the connection pool itself and short-lived query results — every read goes to SQLite (fast enough given FTS5/indices for the "instant" requirement), so there is never a stale in-process copy to invalidate.
- **`signal-player` owns playback state.** The currently loaded track, position, volume, device, gapless queue lookahead, and exclusive-mode flag live in a `RwLock<PlayerState>` inside the `Player` struct. This is the only mutable, non-persisted runtime state of consequence in the backend; it is reconstructed fresh on every startup (nothing about "what was mid-playback" survives a restart except the queue, see below).
- **The frontend never owns state — it mirrors it.** Zustand stores (`usePlayerStore`, `useQueueStore`, `useLibraryFilterStore`, etc.) are populated from IPC responses and kept current by subscribing to bus-derived Tauri events; there is no frontend-side logic that computes playback or library state independently and pushes it backward. This means a second window or a future companion surface could subscribe to the same events and stay in sync for free.
- **No global mutable state anywhere in the backend.** Everything shared crosses thread/task boundaries only via the handles inside `AppState`, each an `Arc<T>` (or `Arc<RwLock<T>>`/`Arc<Mutex<T>>` where interior mutability is required), constructed once in `src-tauri::setup()` and cloned into every command handler and background task that needs it. There are no `static`s holding application data, no `lazy_static`/`OnceCell` singletons standing in for state — `AppState` is the one place all long-lived handles are reachable from.

## Startup sequence

1. `main()` initializes the `tracing` subscriber (stdout layer + the custom `EventBus`-publishing layer) before anything else, so even startup errors are logged.
2. Open the SQLite pool (`signal-db::init`), run pending `sqlx` migrations, enable WAL mode and `foreign_keys = ON`.
3. Construct the `EventBus`.
4. Construct `Player` (`signal-player::Player::new`), which initializes the libmpv handle, applies persisted audio settings (output device, exclusive mode, RG mode) from the `settings` table, and spawns the mpv event thread.
5. Construct `Scanner`, load configured library roots from `settings`, spawn the `notify` watchers for each root (watchers are created but an initial full scan is only kicked off if this is first run or the user has auto-scan-on-launch enabled).
6. Construct `PluginHost`, load enabled plugins from `settings`, subscribe it to the `EventBus`.
7. Assemble `AppState { db, player, scanner, plugins, events }` and register it with Tauri via `.manage(state)`.
8. Restore the persisted queue: `signal-db` loads `queue_items` (persisted on every mutation and on graceful shutdown) and calls `player.restore_queue(items)` so the queue pane is populated immediately, though playback itself does not auto-resume unless the user explicitly enables "resume on launch".
9. Spawn the Tauri event bridge task (subscribes to `EventBus`, forwards to `AppHandle::emit`).
10. Show the main window once steps 2-6 complete; the window is not shown against a half-initialized backend.

## Shutdown

Tauri's `on_window_event(WindowEvent::CloseRequested)` handler intercepts the close, runs an async shutdown routine, and only then allows the window (and process) to actually close:

1. Signal `Scanner` to stop accepting new watcher-triggered jobs and let any in-flight batch finish (bounded by a short timeout; an interrupted scan is safe to resume on next launch since scanning is idempotent and incremental).
2. Flush any buffered `PlayEvent` rows — listening stats are batched in memory for a few seconds to avoid a DB write per second of playback, so shutdown forces an immediate flush of that buffer to `signal-db`.
3. Persist the current queue (`queue_items` table) and current playback position/track so a future "resume on launch" has accurate data even though it's off by default.
4. Stop playback and tell `signal-player` to release the libmpv handle and audio device cleanly (important on exclusive mode, where holding the device prevents other apps from using it).
5. Drop the `EventBus` sender side, which naturally unblocks/ends the bridge task.
6. Close the SQLite pool (checkpoint the WAL) so the database file is left in a clean state.
7. Allow the window close to proceed.

## Key architectural decisions

- **libmpv over rodio/symphonia** — mpv already solves gapless playback, exclusive/hog-mode device access, automatic sample-rate switching, and broad container/codec support as a mature, battle-tested engine; reimplementing that reliably on top of a Rust decode library is a multi-year effort Signal doesn't need to take on.
- **sqlx over diesel** — sqlx's async-native, compile-time-checked raw SQL fits a codebase that wants direct control over FTS5 usage and hand-tuned queries, without diesel's synchronous-by-default model and ORM query-builder abstraction getting in the way.
- **Event bus over direct cross-crate calls** — playback, scanning, and plugins all need to react to the same state changes without knowing about each other; a broadcast bus keeps `signal-player` and `signal-scanner` from taking a dependency on `signal-plugins` (or on `src-tauri`), and keeps the frontend bridge as a single, uniform translation point instead of one bespoke IPC event per producer.
- **FTS5 over tantivy** — FTS5 ships inside SQLite itself (no second index to keep consistent with the source of truth, no extra dependency/binary size), and for a local music library's scale (tens of thousands of tracks, not millions of documents) its performance ceiling is far higher than what the "instant search" requirement actually needs.
- **Six focused crates over one monolith** — `signal-scanner` and `signal-player` in particular have heavy, distinct dependency trees (lofty+notify vs. libmpv FFI); splitting them keeps compile times sane during iteration and makes the dependency-direction rules (see `02-workspace.md`) enforceable by the compiler rather than by convention.
- **Tauri v2 over Electron** — a Rust backend doing real audio/DB/FS work belongs in a native process with a thin webview, not inside a Node runtime; Tauri also gives a materially smaller binary and lower idle memory, which matters for an app meant to sit resident all day.
