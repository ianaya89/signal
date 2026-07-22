# IPC API

This document specifies the Tauri IPC surface between the Rust backend and the React frontend: every command, every event, the shared DTO strategy, and the concurrency guarantees the frontend can rely on. It is the contract layer — if a command or event isn't listed here, it doesn't exist.

## 1. Command catalog

All commands are `snake_case` and domain-prefixed. They are invoked from the frontend through the typed wrapper in `src/lib/ipc/invoke.ts` (see §3), never through raw `@tauri-apps/api/core` `invoke()` calls in components.

### Player

| Command | Args | Returns |
|---|---|---|
| `player_play` | `{ trackId: i64 }` | `Result<(), IpcError>` |
| `player_pause` | `{}` | `Result<(), IpcError>` |
| `player_toggle` | `{}` | `Result<(), IpcError>` |
| `player_seek` | `{ positionMs: u64 }` | `Result<(), IpcError>` |
| `player_next` | `{}` | `Result<(), IpcError>` |
| `player_prev` | `{}` | `Result<(), IpcError>` |
| `player_set_volume` | `{ volume: f32 }` (0.0–1.0) | `Result<(), IpcError>` |
| `player_set_replaygain` | `{ mode: ReplayGainMode }` (`Off \| Track \| Album`) | `Result<(), IpcError>` |

### Queue

| Command | Args | Returns |
|---|---|---|
| `queue_add` | `{ trackIds: i64[], position: QueuePosition }` (`Next \| End`) | `Result<(), IpcError>` |
| `queue_remove` | `{ queueItemId: i64 }` | `Result<(), IpcError>` |
| `queue_move` | `{ queueItemId: i64, toIndex: usize }` | `Result<(), IpcError>` |
| `queue_clear` | `{}` | `Result<(), IpcError>` |
| `queue_list` | `{}` | `Result<Vec<QueueItem>, IpcError>` |
| `queue_save_as_playlist` | `{ name: String }` | `Result<Playlist, IpcError>` |

### Library

| Command | Args | Returns |
|---|---|---|
| `library_scan` | `{ paths: String[] }` | `Result<(), IpcError>` (progress via `scanner:progress`/`scanner:done`) |
| `library_list_albums` | `{ limit: u32, offset: u32, sort: AlbumSort }` | `Result<Page<Album>, IpcError>` |
| `library_list_artists` | `{ limit: u32, offset: u32 }` | `Result<Page<Artist>, IpcError>` |
| `library_get_album` | `{ albumId: i64 }` | `Result<AlbumDetail, IpcError>` (album + tracks) |

### Search

| Command | Args | Returns |
|---|---|---|
| `search_query` | `{ query: String, limit: u32, offset: u32 }` | `Result<Page<Track>, IpcError>` |

### Device

| Command | Args | Returns |
|---|---|---|
| `device_list` | `{}` | `Result<Vec<AudioDevice>, IpcError>` |
| `device_select` | `{ deviceId: String }` | `Result<(), IpcError>` |

### Stats

| Command | Args | Returns |
|---|---|---|
| `stats_overview` | `{ range: StatsRange }` (`Last30Days \| LastYear \| AllTime`) | `Result<StatsOverview, IpcError>` |

### Playlists

| Command | Args | Returns |
|---|---|---|
| `playlist_create` | `{ name: String }` | `Result<Playlist, IpcError>` |
| `playlist_add_tracks` | `{ playlistId: i64, trackIds: i64[] }` | `Result<(), IpcError>` |

### Settings

| Command | Args | Returns |
|---|---|---|
| `settings_get` | `{}` | `Result<Settings, IpcError>` |
| `settings_set` | `{ patch: SettingsPatch }` (partial) | `Result<Settings, IpcError>` |

### Logs

| Command | Args | Returns |
|---|---|---|
| `logs_tail` | `{ lines: u32 }` | `Result<Vec<LogLine>, IpcError>` (backfill; live lines stream via `log:line`) |

### Palette

| Command | Args | Returns |
|---|---|---|
| `palette_execute` | `{ raw: String }` | `Result<PaletteResult, IpcError>` |

## 2. Representative signatures (Rust + TypeScript)

Eight commands, chosen to cover a mutation, a paginated query, a detail fetch, streaming-adjacent state, and the palette dispatcher. Rust signatures live in `src-tauri/src/commands/<domain>.rs`; each pulls `AppState` out of `State<'_, AppState>` and delegates to the relevant crate (`signal-player`, `signal-db`, `signal-search`, `signal-plugins`).

### `player_play`

```rust
// src-tauri/src/commands/player.rs
#[tauri::command]
pub async fn player_play(
    state: State<'_, AppState>,
    track_id: i64,
) -> Result<(), IpcError> {
    state
        .player
        .play(track_id)
        .await
        .map_err(IpcError::from)
}
```

```typescript
// src/lib/ipc/invoke.ts
export async function playerPlay(trackId: number): Promise<void> {
  return invoke("player_play", { trackId });
}
```

### `player_seek`

```rust
#[tauri::command]
pub async fn player_seek(
    state: State<'_, AppState>,
    position_ms: u64,
) -> Result<(), IpcError> {
    state
        .player
        .seek(Duration::from_millis(position_ms))
        .await
        .map_err(IpcError::from)
}
```

```typescript
export async function playerSeek(positionMs: number): Promise<void> {
  return invoke("player_seek", { positionMs });
}
```

### `queue_add`

```rust
#[tauri::command]
pub async fn queue_add(
    state: State<'_, AppState>,
    track_ids: Vec<i64>,
    position: QueuePosition,
) -> Result<(), IpcError> {
    state
        .player
        .queue()
        .add(track_ids, position)
        .await
        .map_err(IpcError::from)
    // emits "queue:changed" on success
}
```

```typescript
export async function queueAdd(
  trackIds: number[],
  position: QueuePosition = "end",
): Promise<void> {
  return invoke("queue_add", { trackIds, position });
}
```

### `library_list_albums`

```rust
#[tauri::command]
pub async fn library_list_albums(
    state: State<'_, AppState>,
    limit: u32,
    offset: u32,
    sort: AlbumSort,
) -> Result<Page<Album>, IpcError> {
    state
        .db
        .list_albums(limit, offset, sort)
        .await
        .map_err(IpcError::from)
}
```

```typescript
export async function libraryListAlbums(
  params: { limit: number; offset: number; sort: AlbumSort },
): Promise<Page<Album>> {
  return invoke("library_list_albums", params);
}
```

### `library_get_album`

```rust
#[tauri::command]
pub async fn library_get_album(
    state: State<'_, AppState>,
    album_id: i64,
) -> Result<AlbumDetail, IpcError> {
    state
        .db
        .get_album_detail(album_id)
        .await
        .map_err(IpcError::from)
}
```

```typescript
export async function libraryGetAlbum(albumId: number): Promise<AlbumDetail> {
  return invoke("library_get_album", { albumId });
}
```

### `search_query`

```rust
#[tauri::command]
pub async fn search_query(
    state: State<'_, AppState>,
    query: String,
    limit: u32,
    offset: u32,
) -> Result<Page<Track>, IpcError> {
    let parsed = signal_search::parse(&query).map_err(IpcError::from)?;
    state
        .db
        .search(parsed, limit, offset)
        .await
        .map_err(IpcError::from)
}
```

```typescript
export async function searchQuery(
  query: string,
  limit: number,
  offset: number,
): Promise<Page<Track>> {
  return invoke("search_query", { query, limit, offset });
}
```

### `device_select`

```rust
#[tauri::command]
pub async fn device_select(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), IpcError> {
    state
        .player
        .select_device(&device_id)
        .await
        .map_err(IpcError::from)
    // emits "player:device-changed" on success
}
```

```typescript
export async function deviceSelect(deviceId: string): Promise<void> {
  return invoke("device_select", { deviceId });
}
```

### `palette_execute`

```rust
#[tauri::command]
pub async fn palette_execute(
    state: State<'_, AppState>,
    raw: String,
) -> Result<PaletteResult, IpcError> {
    let command = signal_core::palette::parse(&raw).map_err(IpcError::from)?;
    signal_core::palette::dispatch(command, &state)
        .await
        .map_err(IpcError::from)
}
```

```typescript
export async function paletteExecute(raw: string): Promise<PaletteResult> {
  return invoke("palette_execute", { raw });
}
```

## 3. Shared DTO strategy

Every DTO crossing the IPC boundary is defined once, in Rust, in `signal-core::dto`. Structs derive `Serialize`/`Deserialize` with `#[serde(rename_all = "camelCase")]` so JSON on the wire is camelCase without any manual renaming in TypeScript.

```rust
// signal-core/src/dto/track.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub artist_id: i64,
    pub album_id: Option<i64>,
    pub track_number: Option<u32>,
    pub duration_ms: u64,
    pub technical: TrackTechnical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackTechnical {
    pub codec: String,
    pub container: String,
    pub bitrate_kbps: Option<u32>,
    pub bit_depth: Option<u8>,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub replaygain_track_gain: Option<f32>,
    pub replaygain_album_gain: Option<f32>,
    pub peak: Option<f32>,
    pub dr_score: Option<u8>,
    pub encoder: Option<String>,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub md5: Option<String>,
}
```

The equivalent hand-written TypeScript interface lives in `src/lib/ipc/types.ts`:

```typescript
// src/lib/ipc/types.ts
export interface Track {
  id: number;
  title: string;
  artistId: number;
  albumId: number | null;
  trackNumber: number | null;
  durationMs: number;
  technical: TrackTechnical;
}

export interface TrackTechnical {
  codec: string;
  container: string;
  bitrateKbps: number | null;
  bitDepth: number | null;
  sampleRateHz: number;
  channels: number;
  replaygainTrackGain: number | null;
  replaygainAlbumGain: number | null;
  peak: number | null;
  drScore: number | null;
  encoder: string | null;
  filePath: string;
  fileSizeBytes: number;
  md5: string | null;
}
```

**Keeping the two in sync (MVP).** `signal-core::dto` is the source of truth; TypeScript types are maintained by hand in `src/lib/ipc/types.ts`, mirroring field names/nullability field-for-field. `i64`/`u64` map to `number` (ids are auto-increment SQLite rowids, safely below `Number.MAX_SAFE_INTEGER`). `Option<T>` maps to `T | null`. A comment block at the top of `types.ts` lists the Rust source file each interface mirrors, so a diff to `signal-core/src/dto/*.rs` is a visible prompt to update the TS side. There is no build-time drift check in the MVP; a mismatch fails at runtime as a `TypeError`, and PR review is the enforcement mechanism.

**Post-MVP.** Once the DTO surface stabilizes, switch to `ts-rs` (derive `#[derive(TS)]`, emit `.d.ts` in a build step) or `specta` (also generates the typed `invoke` wrapper itself via `tauri-specta`, eliminating the hand-written functions in `invoke.ts`/`events.ts`). Deferred deliberately — codegen before the command surface stabilizes generates more churn than it saves.

## 4. IpcError design

All commands return `Result<T, IpcError>`. `IpcError` is a serializable enum with a `kind` discriminant and a human-readable `message`, so the frontend can pattern-match on `kind` without parsing strings.

```rust
// signal-core/src/error.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub kind: IpcErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IpcErrorKind {
    NotFound,
    InvalidArgument,
    Io,
    Database,
    Decode,
    DeviceUnavailable,
    PlaybackFailed,
    ParseError,
    PluginError,
    Internal,
}

impl From<signal_db::DbError> for IpcError {
    fn from(err: signal_db::DbError) -> Self {
        let kind = match &err {
            signal_db::DbError::NotFound(_) => IpcErrorKind::NotFound,
            signal_db::DbError::Sqlx(_) => IpcErrorKind::Database,
        };
        IpcError { kind, message: err.to_string() }
    }
}
```

Tauri serializes the `Err` variant of a `Result` returned from a command as the rejection value of the JS-side promise, so `invoke()` throws an object shaped like `IpcError` directly — no wrapping required.

```typescript
// src/lib/ipc/types.ts
export type IpcErrorKind =
  | "notFound"
  | "invalidArgument"
  | "io"
  | "database"
  | "decode"
  | "deviceUnavailable"
  | "playbackFailed"
  | "parseError"
  | "pluginError"
  | "internal";

export interface IpcError {
  kind: IpcErrorKind;
  message: string;
}

export function isIpcError(err: unknown): err is IpcError {
  return (
    typeof err === "object" &&
    err !== null &&
    "kind" in err &&
    "message" in err
  );
}
```

**Mapping kinds to toasts.** A single handler wraps every mutation and maps `kind` to a toast variant and copy. Read paths (TanStack Query) surface the same mapping through their `onError`.

```typescript
// src/lib/ipc/errors.ts
const TOAST_COPY: Record<IpcErrorKind, { title: string; variant: "destructive" | "default" }> = {
  notFound: { title: "Not found", variant: "destructive" },
  invalidArgument: { title: "Invalid input", variant: "destructive" },
  io: { title: "Filesystem error", variant: "destructive" },
  database: { title: "Database error", variant: "destructive" },
  decode: { title: "Could not decode file", variant: "destructive" },
  deviceUnavailable: { title: "Output device unavailable", variant: "destructive" },
  playbackFailed: { title: "Playback failed", variant: "destructive" },
  parseError: { title: "Could not parse command", variant: "default" },
  pluginError: { title: "Plugin error", variant: "destructive" },
  internal: { title: "Unexpected error", variant: "destructive" },
};

export function reportIpcError(err: unknown): void {
  if (!isIpcError(err)) {
    toast({ title: "Unexpected error", description: String(err), variant: "destructive" });
    return;
  }
  const copy = TOAST_COPY[err.kind];
  toast({ title: copy.title, description: err.message, variant: copy.variant });
}
```

`parseError` is deliberately `variant: "default"` — bad palette/search syntax is user-correctable, not a system failure, and shouldn't read as alarming.

## 5. Event catalog

Events are backend -> frontend, pushed via Tauri's event system and subscribed to once at startup by `src/lib/ipc/events.ts` (see `docs/06-frontend.md` §4 for the bridge). Every event has a Rust payload struct in `signal-core::dto::event` and a matching TS interface in `src/lib/ipc/types.ts`.

| Event | Payload | Emission |
|---|---|---|
| `player:state` | `PlayerStateChanged` | On every play/pause/stop/track-end transition |
| `player:progress` | `PlayerProgress` | Throttled to 4 Hz (every 250ms) while playing |
| `player:track-changed` | `TrackChanged` | On track boundary (manual skip or natural advance) |
| `player:device-changed` | `DeviceChanged` | After `device_select` resolves, or on device hot-plug/removal |
| `scanner:progress` | `ScannerProgress` | Throttled to ~5 Hz during `library_scan` |
| `scanner:done` | `ScannerDone` | Once, at scan completion (success or partial failure) |
| `queue:changed` | `QueueChanged` | After any mutation to the queue (add/remove/move/clear) |
| `log:line` | `LogLine` | Per emitted log line, unthrottled (log volume is low; UI ring-buffers to last N lines) |

```rust
// signal-core/src/dto/event.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStateChanged {
    pub state: PlayerState, // Stopped | Playing | Paused
    pub track_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProgress {
    pub track_id: i64,
    pub position_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackChanged {
    pub track: Track,
    pub queue_item_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceChanged {
    pub device: AudioDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannerProgress {
    pub files_scanned: u32,
    pub files_total: Option<u32>, // None until the walk finishes counting
    pub current_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannerDone {
    pub tracks_added: u32,
    pub tracks_updated: u32,
    pub tracks_removed: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueChanged {
    pub items: Vec<QueueItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub level: LogLevel, // Trace | Debug | Info | Warn | Error
    pub target: String,
    pub message: String,
    pub timestamp_ms: u64,
}
```

```typescript
// src/lib/ipc/types.ts
export interface PlayerStateChanged {
  state: "stopped" | "playing" | "paused";
  trackId: number | null;
}

export interface PlayerProgress {
  trackId: number;
  positionMs: number;
  durationMs: number;
}

export interface TrackChanged {
  track: Track;
  queueItemId: number | null;
}

export interface DeviceChanged {
  device: AudioDevice;
}

export interface ScannerProgress {
  filesScanned: number;
  filesTotal: number | null;
  currentPath: string;
}

export interface ScannerDone {
  tracksAdded: number;
  tracksUpdated: number;
  tracksRemoved: number;
  errors: string[];
}

export interface QueueChanged {
  items: QueueItem[];
}

export interface LogLine {
  level: "trace" | "debug" | "info" | "warn" | "error";
  target: string;
  message: string;
  timestampMs: number;
}
```

`player:progress` at 4 Hz is a deliberate ceiling: smooth-looking seekbar, never a re-render bottleneck. The backend throttles at the emission site inside `signal-player`'s playback loop rather than relying on the frontend to debounce, so unused ticks never cross the IPC boundary.

## 6. Streaming and large data

### Pagination

Every list-returning command (`library_list_albums`, `library_list_artists`, `search_query`) uses the same envelope:

```rust
// signal-core/src/dto/page.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
}
```

```typescript
export interface Page<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}
```

`limit`/`offset` are plain integers, not cursors — the underlying tables are indexed SQLite with stable sort keys (`sort_key`, `id`), so offset pagination is cheap even at 100k+ rows, and it keeps the TanStack Query cache key trivial: `['albums', { sort, limit, offset }]`. `TrackTable` (see `docs/06-frontend.md`) pages through this in windowed chunks sized to its virtualization viewport, not the full 100k rows at once.

### Artwork

Album art is never sent as a base64 blob over IPC — at 100k+ tracks this would blow out both IPC message size and the React render tree (every `<img>` would need a fresh data URI on each query result). Instead, artwork is served through a custom URI scheme registered on the Tauri builder, and the frontend just points `<img src>` at it like any other URL.

```rust
// src-tauri/src/main.rs
use tauri::{http::Response, UriSchemeContext};

fn artwork_protocol_handler(
    ctx: UriSchemeContext<'_, tauri::Wry>,
    request: http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let state = ctx.app_handle().state::<AppState>();
    let album_id: i64 = match request
        .uri()
        .path()
        .trim_start_matches('/')
        .parse()
    {
        Ok(id) => id,
        Err(_) => return Response::builder().status(400).body(vec![]).unwrap(),
    };

    match state.db.get_artwork_blocking(album_id) {
        Some(bytes) => Response::builder()
            .status(200)
            .header("Content-Type", "image/jpeg")
            .header("Cache-Control", "public, max-age=31536000, immutable")
            .body(bytes)
            .unwrap(),
        None => Response::builder().status(404).body(vec![]).unwrap(),
    }
}

fn main() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("signal", |ctx, request| {
            artwork_protocol_handler(ctx, request).map(Into::into)
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![/* ... */])
        .run(tauri::generate_context!())
        .expect("error while running signal");
}
```

```tsx
// src/components/library/AlbumCard.tsx
<img
  src={`signal://artwork/${album.id}`}
  alt={album.title}
  loading="lazy"
  className="aspect-square object-cover"
/>
```

The `Cache-Control: immutable` header lets the WebView cache art indefinitely; if artwork changes (re-tag, re-scan), the scanner bumps an `artwork_version` column and the frontend appends `?v=<version>` to bust the cache, rather than the protocol handler doing invalidation bookkeeping itself.

## 7. `palette_execute` contract

The command line and command palette both funnel non-trivial input through a single command, `palette_execute`, which takes the raw string the user typed and returns a discriminated result:

```rust
// signal-core/src/dto/palette.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "payload")]
pub enum PaletteResult {
    Navigate(NavigateTarget),
    Feedback(FeedbackMessage),
    Error(PaletteError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateTarget {
    pub route: String, // e.g. "/albums/482"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackMessage {
    pub message: String, // e.g. "Queued 12 tracks"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteError {
    pub message: String,
    pub suggestion: Option<String>, // e.g. closest matching command name
}
```

```typescript
export type PaletteResult =
  | { kind: "navigate"; payload: { route: string } }
  | { kind: "feedback"; payload: { message: string } }
  | { kind: "error"; payload: { message: string; suggestion: string | null } };
```

Backend parsing lives in `signal-core::palette` and covers commands that mutate or query backend state directly (`play bocanada`, `device topping`, `scan ~/Music`). Pure client-side navigation (`:albums`, `:artists`, `g a`) resolves before ever reaching IPC — see the split in `docs/06-frontend.md` §8. `Navigate` results call `router.navigate({ to: payload.route })`; `Feedback` renders a transient toast; `Error` renders inline in the palette input with `suggestion` as a "did you mean" hint.

## 8. Concurrency and ordering guarantees

- **Runtime.** All commands run on the Tauri-managed Tokio runtime as `async fn`, never blocking the main WebView thread. Blocking work (e.g. artwork disk reads in the URI protocol handler) uses `spawn_blocking` or an explicit sync path, since the protocol handler itself is not async.
- **Player command serialization.** All player mutations (`player_play`, `player_pause`, `player_seek`, `player_next`, `player_prev`, `queue_*`) are serialized through a single mpsc channel owned by `signal-player`'s playback actor, processed one at a time in submission order — a `player_seek` issued right after `player_play` cannot race it and get silently dropped. Read-only commands (`queue_list`, `library_*`, `search_query`) skip this channel and run concurrently with player commands.
- **Command responses vs. events.** A command's `Result` resolving successfully means the backend accepted and applied the mutation — it does **not** guarantee the corresponding event has reached the frontend yet, since events dispatch asynchronously after the channel processes the command. Events for different commands can arrive interleaved or out of order relative to when their originating commands resolved.
- **Frontend rule.** Because of this, the frontend never treats a resolved command promise as final state: **use the command response only for optimistic, immediate UI feedback; treat the next matching event as the source of truth.** `playerStore` doesn't set `isPlaying = true` from `player_play`'s resolution — it waits for `player:state`. `queueStore` doesn't splice an item from `queue_remove`'s resolution — it replaces its entire list from the next `queue:changed` payload. This keeps stores consistent under reordering: whichever event arrives last wins, and it always carries a complete snapshot, never a delta.
