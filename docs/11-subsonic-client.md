# Subsonic Client

Signal already embeds an OpenSubsonic-compatible server (`signal-server`, see its module doc and [05-ipc-api.md](./05-ipc-api.md) §"Mobile server") so phone apps like Symfonium and DSub can reach into the local library. This document specifies the mirror-image feature: Signal acting as a Subsonic/OpenSubsonic **client**, browsing and streaming from remote servers — another Signal instance, Navidrome, Airsonic, Gonic, or anything else speaking the same protocol — from inside the same desktop app.

This slots into the roadmap as **M6 — Subsonic Client**, immediately after M5 (`docs/07-roadmap.md` currently runs M0–M5; M6 is the next open slot). It depends on M2 (`signal-player` exists) and reuses the DTO/auth work `signal-server` already did for the inbound direction, but every piece of the outbound (client) path is new: a new crate, a `Player` API change, new IPC commands, and new UI surface.

## Implementation status

*(as of 2026-08-04)* Phases 1–4 are implemented and passing all tests (`cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean, `cargo test --workspace` — 28 test binaries, zero failures — `tsc --noEmit` clean, `vite build` succeeds). Phase 5 has not started.

| Phase | Status |
|---|---|
| 1 — Shared DTOs + `signal-subsonic-client` crate | Done |
| 2 — Player `MediaSource` support | Done |
| 3 — `src-tauri` commands + settings storage | Done |
| 4 — UI | Done |
| 5 — Polish | Not started |

The feature is fully exercisable today from the UI — add a remote source in settings, browse its artists and albums, and stream a track into `signal-player` alongside the local library — matching most of the Phase 4 "shippable state" this document anticipated (§3), except for queue visibility (see below). A handful of implementation details turned out to differ from what's specified below; each is called out inline where it applies, and summarized here:

- The server's response envelope (§2.2) was not refactored onto the shared `Envelope<T>` type; that type exists client-side only, for parsing.
- `ClientError::Parse` carries a `String`, not a `serde_json::Error` (§2.1).
- `SignalError::Remote` has no `From<ClientError>` impl; conversion happens via a `RemoteResultExt` trait in the command module instead (§3, Phase 3).
- `ArtistID3` gained an optional `cover_art` field beyond what §2.2 originally specified (§2.2).
- `remote_play` identified the in-flight track to the player with a synthetic id of `-1` (§3, Phase 3) — a stopgap Phase 4 replaced (see below).
- Third-party test fixtures (§7) are hand-authored from each server's documented/observed shape, not captured from a live instance — the manual Navidrome smoke test remains required and has not yet been performed.
- `remote_play`'s Phase 3 stopgap id (`REMOTE_TRACK_ID`, always `-1`) is gone (§3, Phase 4). `remote_play` now takes the full `Child` song rather than just an id, since there's no `tracks` row to recover title/artist/album from later; a new `remote_play_context(sourceId, songs, startIndex)` command plays one song with the rest as follow-on order. The remote command count went from 12 to 13.
- New mechanism, not in the plan: a negative-id registry, `RemoteSlab` (`src-tauri/src/state.rs`). Registering remote songs under ids from a disjoint negative range lets the entire advance path (`PlayContext`, `play_history`, `next_candidate`, `consume`) ride the existing `i64`-keyed machinery unchanged; only `commands::player::start_track`, `autoplay::restage`, and `commands::library::library_get_track` branch on whether an id is remote. Real ids come from SQLite `AUTOINCREMENT` (starting at 1), so the ranges cannot collide, and entries are never evicted, since `play_history` holds ids indefinitely and reusing one would resume the wrong song. Covered by 4 new unit tests in `src-tauri/src/state.rs`.
- `library_get_track` now branches on a negative id and returns a synthetic `TrackWithContext` assembled from the `RemoteSlab` — the change that made `TransportBar`, `MiniPlayer`, `ChainView`, `InspectorPane` and `AlbumsView` display remote tracks correctly with zero frontend edits, since all five already called `getTrack(trackId)`. The synthetic `Track` carries `-1` for `artistId`/`albumId`, an empty `file_path`, and `0` for `sample_rate_hz`/`channels`, since the Subsonic wire format doesn't carry those.
- Queue integration for remote tracks was not delivered as §3, Phase 4 described — see the inline note on that bullet.
- Cover art shipped without any CSP change — see the inline note on that bullet, and §6.
- Risk 5's `https://` half (§5) is still unverified; only the `http://` loopback case — now including remote-to-remote gapless advance — has been tested so far.

## 1. Goal and non-goals

**Goal:** from inside Signal, add one or more remote OpenSubsonic servers, browse their artists/albums/songs, and stream their tracks through the same `signal-player`/queue/now-playing UI used for the local library — without pretending those tracks are part of the local library.

**Non-goals for v1:**

- **No catalog sync.** Remote artists/albums/tracks are never written into `tracks`/`albums`/`artists`. Every browse is a live request to the remote server; nothing is cached into `signal-db` as if it were locally scanned content.
- **No offline cache.** Every remote play is a live stream. No remote audio is ever written to local disk, no local file backs a remote track.
- **No transcoding requests.** The client always requests the raw `stream` endpoint with no `maxBitRate`/`format` hints, mirroring `signal-server`'s own MVP stance (`media.rs`: "no transcode, v1"). Whatever the remote serves, mpv plays as-is.
- **No write-back beyond scrobbling.** No remote star/rating/playlist editing in v1 — read and stream only, plus telling the remote server a track played (§3.5 and §5, Phase 5).
- **No aggregated multi-server view.** Each configured remote source is its own browsable tree (a sidebar entry per source), not merged with the local library or with each other into one search/browse surface.
- **No cross-device credential sync.** Each Signal install stores its own remote source rows locally; nothing about this feature talks to a sync service.

## 2. Architecture decisions

### 2.1 New crate: `signal-subsonic-client`

Signal's dependency-direction rule (`01-architecture.md`, `02-workspace.md`) is one crate per resource layer, all rooted at `signal-core`. `signal-server` already owns "speak Subsonic as a server" — it wraps `axum`, owns HTTP routing, and answers requests. The client side wraps `reqwest` (already a workspace dependency at `rustls-tls` + `json`, `Cargo.toml:65`) and *makes* requests; it has no reason to depend on `axum`/`tower-http`, and `signal-server` has no reason to depend on `reqwest`. Splitting them keeps each crate's dependency tree — and compile time — proportional to what it actually does, the same reasoning that already split `signal-scanner` (lofty+notify) from `signal-player` (libmpv FFI).

`signal-subsonic-client` owns:
- An HTTP client wrapper around `reqwest::Client`, one instance per configured remote source (so per-source TLS settings, §2.4, don't leak across sources).
- Auth token/salt generation (§2.4).
- Typed request methods: `ping`, `get_artists`, `get_artist`, `get_album`, `search3`, plus **URL builders** (no network call) for `stream` and `getCoverArt`, since those are consumed as plain URLs by mpv and `<img>` respectively, not fetched by this crate itself.
- A `ClientError` (`thiserror`) enum: `Http(reqwest::Error)`, `Auth`, `Api { code: u32, message: String }` (a Subsonic error envelope came back), `Parse(serde_json::Error)`. **Shipped as:** `Parse(String)` instead — the variant also needs to carry "server said ok but sent no payload" and non-JSON HTML error pages, neither of which fits `serde_json::Error`, and it quotes a truncated snippet of the offending body, the most useful thing for diagnosing a wrong base URL.

It depends on `signal-subsonic-types` (below) and `reqwest`; it does **not** depend on `signal-db`, `signal-core`, or `signal-player` — a remote browse result is a plain DTO, and turning it into something `signal-player`/the queue understands is `src-tauri`'s job (§2.3), keeping this crate a pure protocol client that could in principle be reused outside Signal.

### 2.2 Shared DTOs: a new `signal-subsonic-types` crate, not `signal-core`

`signal-server/src/dto.rs` already defines `Child`, `AlbumID3`, and `ArtistID3` — exactly the wire shapes a client needs to *deserialize*. The question is where the shared definition should live so both `signal-server` (serialize) and `signal-subsonic-client` (deserialize) use the same struct without either depending on the other.

**Decision: a new crate, `signal-subsonic-types`, not `signal-core`.** `signal-core` is described in `01-architecture.md` as Signal's own domain model — "depended on by every crate above, depends on none of them" — `Track`, `Album`, `PlayerState`, `SignalEvent`. Subsonic's wire format is a *foreign protocol's* shape, not Signal's domain: its field names, optionality, and envelope conventions are dictated by an external spec Signal doesn't control (and OpenSubsonic extensions evolve independently of Signal's own data model). Folding `Child`/`AlbumID3`/`ArtistID3` into `signal-core` would make the one crate every other crate depends on also carry protocol concerns it doesn't own, and would couple unrelated things: a change to how Signal privately models a `Track` has no reason to touch the Subsonic wire format, and vice versa. A dedicated crate keeps "what does an OpenSubsonic response look like on the wire" as its own single-owner resource layer, consistent with the existing convention, and it's cheap: `signal-subsonic-types` needs only `serde`/`serde_json`/`chrono`, so it adds negligible compile time to either consumer.

What moves and what stays:
- **Moves:** the plain struct definitions — `Child`, `AlbumID3`, `ArtistID3` — plus a new `Playlist` struct (today `signal-server/src/handlers/playlists.rs` builds its playlist payload ad hoc via `serde_json::json!` in `playlist_attrs()`; giving it a real struct in the shared crate is a small refactor bundled into Phase 1 so the client gets typed playlist browsing for free later), an `ApiErrorDto { code: u32, message: String }` for the `error` envelope field, and a generic `Envelope<T>` wrapper matching what `envelope.rs::render()` currently builds by hand (`status`, `version`, `type`, `serverVersion`, `openSubsonic`, `error`, plus `#[serde(flatten)]` for the endpoint-specific payload).
- **Stays in `signal-server`:** the `Sid` type (`ids.rs`) and every `from_track`/`from_summary` mapper. `Sid`'s `"tr-7"`/`"al-3"` prefix scheme is how *Signal's own server* encodes its internal integer ids — it has no bearing on how a remote server ids its own tracks (a Navidrome id might be a bare integer string, an Airsonic-compatible server might use a UUID). Client-side, every id field is just an opaque `String`, which is already how the DTOs declare `id` today — no change needed there. The `from_track(&Track, &NameMaps)` constructors depend on `signal_core::Track` and `signal-server`-internal lookup maps; moving them would pull `signal-core` into `signal-subsonic-types`, exactly the coupling this split avoids. `signal-server` keeps building shared-type instances from its own domain data; `signal-subsonic-client` only ever deserializes them.

**Implementation note — the envelope refactor didn't happen.** `signal-server/src/envelope.rs::render()` still builds the response envelope by hand with `serde_json::json!`, not the shared `Envelope<T>` type described above. Its payload key varies per endpoint and the XML walker consumes the resulting `Value` directly, so retargeting it onto a generic struct wasn't worth the churn for something already working. `Envelope`/`ResponseBody` shipped in the shared crate anyway, but for *parsing* only — the client deserializes into them, with a `ResponseBody::take::<T>(key)` accessor standing in for the generic flatten this section originally proposed. Server-side envelope refactor is deferred as unnecessary rather than abandoned.

One real wrinkle found while reading `dto.rs`: `Child::kind` is declared `pub kind: &'static str` (always `"music"`, since Signal has no other content types) — that's fine to *serialize* but can't derive `Deserialize` as written, since a `'static str` can't borrow from arbitrary response bytes. The shared `Child` needs `kind: String` (or a `#[serde(other)]`-tolerant enum, since some servers return `"podcast"`/`"video"` for other content Signal doesn't care about and should just ignore). This is a two-line change but worth calling out as the one non-mechanical part of the extraction.

**Also added beyond the plan:** `ArtistID3` gained an optional `cover_art` field. Signal's own server always leaves it `None` (it has no per-artist cover art concept), but real third-party servers populate it, and a shared shape without the field would fail to deserialize their responses.

The bigger asymmetry: **`signal-server` always populates every field it serializes** (it's generating the JSON from a known-complete `Track` row), so today's DTOs have few `Option`s and no `#[serde(default)]`. A client deserializing a *third-party* server's response can't assume the same completeness — real Navidrome/Airsonic/Gonic responses disagree on which optional OpenSubsonic fields they actually populate. **Decision:** add `#[serde(default)]` liberally across the shared structs' optional-ish fields (bitrate, genre, cover art id, rating, etc.) rather than maintaining two parallel struct definitions (a strict server-side one and a lax client-side one) — that would defeat the point of sharing them. This costs `signal-server` nothing (its own generator always populates the fields, so the default never triggers) and buys the client tolerance for the real-world variance it will actually see. Phase 1's test fixtures (§7) should include at least one captured real third-party response, not just Signal's own shape, specifically to validate this.

### 2.3 Track identity: no local row, composite reference

`tracks.file_path TEXT NOT NULL UNIQUE` (`03-database-schema.md` §2) encodes a hard invariant relied on throughout `signal-scanner`, `signal-analysis`, and `signal-server`'s own `stream` handler: every row in `tracks` is a file Signal can find on disk. A remote track has no such file — shoehorning it into `tracks` via a nullable `file_path` or a synthetic sentinel path would break that invariant for every consumer that currently assumes it, for a feature whose v1 explicitly doesn't want catalog sync anyway. So remote tracks get **no row in `tracks`, ever**, in v1.

Instead, a remote track's identity is a composite the UI carries around:

```rust
// signal-core — new, alongside the existing domain types in models/
pub struct RemoteTrackRef {
    pub source_id: i64,   // remote_sources.id
    pub remote_id: String, // the opaque song id the remote server assigned
}
```

Playback needs a discriminated union so a queue entry (or a bare "play this now" call) can be either kind:

```rust
// signal-core
pub enum PlaybackSource {
    Local { track_id: i64 },
    Remote { source_id: i64, remote_id: String },
}
```

**v1 scope cut:** `queue_items` (`03-database-schema.md` §2) has `track_id INTEGER NOT NULL REFERENCES tracks(id)` and is persisted so the queue survives a restart (M3 exit criterion). Making that table admit remote entries needs either a nullable `track_id` plus nullable `remote_source_id`/`remote_id` columns and a `CHECK` enforcing exactly one populated, or a parallel table — real schema surgery on a table every existing queue path touches. Given the DB-changes brief for this milestone is deliberately narrow (§4), v1 keeps remote queue entries **runtime-only**: they live in `signal-player`'s/`src-tauri`'s in-memory queue representation alongside `PlaybackSource::Local` items, but are not written to `queue_items`, so a remote track queued for later does not survive an app restart. This is an explicit, visible tradeoff — called out again in §5 and §6 — not an oversight; persisting mixed-source queues is a reasonable follow-up once the read/stream path has shipped and it's clear which servers people actually want persistent queues against.

### 2.4 Player: `MediaSource` instead of a bare `PathBuf`

Today `signal-player`'s command enum takes a path outright:

```rust
// crates/signal-player/src/player.rs (current)
pub(crate) enum Cmd {
    Load { track_id: i64, path: PathBuf },
    LoadAt { track_id: i64, path: PathBuf, position_ms: u64 },
    SetNext { track_id: i64, path: PathBuf },
    ...
}
```

and `engine.rs::apply()` turns it into a string for `mpv.command("loadfile", &[&path_str, ...])`. mpv's `loadfile` already accepts a URL as-is — the demux/protocol layer is ffmpeg's, and ffmpeg's `http`/`https` protocols don't care whether the string came from a local path or a network location. The change needed is purely at the type level: replace `path: PathBuf` with `source: MediaSource` on `Load`, `LoadAt`, and `SetNext`, and thread the enum through to where the string gets built:

```rust
// signal-core — public, since both signal-player's API and src-tauri's
// queue/remote-play command construction need to name this type
pub enum MediaSource {
    File(PathBuf),
    Url(String),
}
```

`engine.rs::apply()`'s three `Cmd::Load*`/`SetNext` arms collapse their `path.to_string_lossy().into_owned()` line into a two-arm match producing the same `String` either way; everything downstream (the `mpv.command("loadfile", &[...])` calls, the gapless window bookkeeping in `self.window`, `drop_next_entries`) is unchanged, because mpv's playlist doesn't care what kind of string it was handed. That's a genuinely nice property of this design: gapless prefetch (`Cmd::SetNext`, the 2-slot playlist window described in `engine.rs`'s module doc) needs no special-casing for remote tracks at the mpv layer — the risk is real-world latency, not code complexity (§5).

`Player`'s public methods (`load_and_play`, `load_paused_at`, `set_next`) take `MediaSource` in place of `PathBuf`; `signal_core::MediaSource` is the right home for the type (not `signal-player`-private) because `src-tauri` needs to construct one from either a local `Track.technical.file_path` or a `remote_id` + stream-URL builder without a build-time coupling issue — `signal-core` is already the layer every crate can see.

### 2.5 Auth: per-request salted token, mirroring `signal-server`'s own check

`signal-server/src/auth.rs::check()` validates `t=md5(password+salt)&s=salt` (preferred) or `p=` (plaintext or `enc:`-prefixed hex). The client side needs to *produce* exactly that pair. Salt generation deliberately avoids adding a `rand` dependency — the workspace doesn't have one today, and `signal-server/src/handlers/lists.rs` already made the same call for its `getRandomSongs`/`random` shuffle, using a time-seeded LCG rather than pulling in `rand` (its comment: "same no-RNG-dependency stance as `PlayContext::peek_next`'s shuffle jitter"). A Subsonic auth salt doesn't need cryptographic randomness either — it's an anti-replay nonce, not a secret — so `signal-subsonic-client` follows the same convention: seed from `SystemTime::now()` + a per-client counter, hex-encode a few bytes. Token computation itself reuses the workspace's existing `md-5`/`hex` crates, the same two dependencies `signal-server/src/auth.rs` already uses — so the exact byte sequence (`md5(password.as_bytes() ++ salt.as_bytes())`, hex-encoded) is identical on both ends, which is what makes the self-test in §7 exact rather than approximate.

**Password storage:** a plaintext `password` column on the new `remote_sources` table (§4). This is not a new risk class for the app: `server.password` (the embedded server's own shared password) is already stored as plaintext in the `settings` table today (`src-tauri/src/commands/server.rs::read_config`), so remote-source credentials land at the same trust level the app already operates at, not below it. It's worth flagging anyway (§5) because these are credentials for *someone else's* account/server, often reused elsewhere, which raises the stakes of that row leaking even though the mechanism is unchanged. OS keyring integration (`keyring` crate, or a Tauri keyring/stronghold plugin) is recommended as a stretch item — realistically Phase 5 or later, gated on it being cheap enough not to block the read/stream path from shipping.

## 3. Phased task breakdown

Each phase ships something independently testable/demoable, following the roadmap's own "no long-lived everything-is-broken branches" rule.

### Phase 1 — Shared DTOs + `signal-subsonic-client` crate

- New workspace members: `signal-subsonic-types` (protocol structs, §2.2) and `signal-subsonic-client` (HTTP wrapper, §2.1).
- Extract `Child`, `AlbumID3`, `ArtistID3` from `signal-server/src/dto.rs` into `signal-subsonic-types`, add `Deserialize`, loosen per §2.2 (`kind: String`, `#[serde(default)]` on optionals). Add the new `Playlist` struct and `ApiErrorDto`/`Envelope<T>`. Update `signal-server` to build these shared structs instead of its private copies (its `from_track`/`from_summary` mappers stay put, just target the shared types).
- `signal-subsonic-client` methods: `ping`, `get_artists`, `get_artist(id)`, `get_album(id)`, `search3(query, ...)`, plus non-network URL builders `stream_url(remote_id)` and `cover_art_url(remote_id, size)`.
- Auth module implementing §2.4's salt+token generation, with unit tests asserting byte-identical output to `signal-server::auth`'s own test vectors for the same `(password, salt)` pair.
- Unit tests deserializing canned JSON fixtures: Signal's own server shape (trivial — round-trips what `signal-server`'s existing DTO tests already assert) *and* at least one captured real Navidrome and one real Airsonic (or Gonic) response, stored under `crates/signal-subsonic-client/tests/fixtures/`, mirroring the repo's existing `fixtures/` convention for audio test files (`02-workspace.md`).
- Integration test (also lands in this phase, see §7): spin up `signal_server::start()` in-process on an ephemeral port, point `SubsonicClient` at it, exercise the full read path end-to-end.

**Shippable state:** a library crate with no UI/IPC surface yet, fully unit- and integration-tested against both a real embedded server and third-party fixtures.

### Phase 2 — Player `MediaSource` support

- `signal-core`: add `MediaSource` (§2.4).
- `signal-player`: `Cmd::Load`/`LoadAt`/`SetNext` and the corresponding `Player` methods take `MediaSource` instead of `PathBuf`; `engine.rs::apply()`'s three arms build the mpv target string via a two-line match instead of `path.to_string_lossy()`. No change to `apply_audio`, `handle_event`, `drop_next_entries`, or the gapless window bookkeeping — the whole point of putting the enum this low is that mpv, and therefore the rest of the engine, doesn't need to know or care which variant it got.
- Update every existing call site (`src-tauri`'s player/queue commands) to wrap local paths in `MediaSource::File(..)` — mechanical, no behavior change for local playback.
- New integration test: serve a `fixtures/` audio file over loopback HTTP (either via a throwaway `signal-server` instance, reusing the Phase 1 test harness, or a minimal standalone static file server), drive `Player::load_and_play(id, MediaSource::Url(url))`, assert the state transitions to `Playing` and a non-zero `duration_ms` is observed within a timeout — the same shape as the existing `tests/gapless.rs`-style tests but over a URL.

**Shippable state:** `signal-player` can play a URL; verified once by hand that mpv actually opens an `http://127.0.0.1:<port>/...` stream in dev builds, since this is the first time the app has asked mpv to touch the network (see Risk: bundled libmpv network support, §5).

### Phase 3 — `src-tauri` commands + settings storage

- Migration `0007_remote_sources.sql` (§4).
- `signal-db`: `RemoteSourceRepo`, mirroring the existing repository shape (`SettingsRepo`, `TrackRepo`, etc.) — `list`, `get`, `create`, `update`, `delete`, `record_ping` (updates `last_ping_at`/`last_ping_ok` after a connection test).
- `signal_core::SignalError` gets a `Remote(String)` variant plus a `From<signal_subsonic_client::ClientError>` impl, following the existing "one `From` impl per crate error, converted to an owned `String`" pattern (`01-architecture.md` §"Error handling strategy"). **Implementation note:** the `From` impl doesn't exist. `signal-core` is "depended on by every crate above, depends on none of them" (§2.2, quoting `01-architecture.md`), and a `From<signal_subsonic_client::ClientError>` impl on `signal_core::SignalError` would force `signal-core` to depend on `signal-subsonic-client`, inverting that rule. Instead a `RemoteResultExt` trait in `src-tauri/src/commands/remote.rs` performs the conversion at the call site, matching the existing `DbResultExt`/`PlayerResultExt` pattern already used there for `signal-db` and `signal-player` errors.
- New `src-tauri/src/commands/remote.rs`:

  | Command | Args | Returns |
  |---|---|---|
  | `remote_source_add` | `{ name, baseUrl, username, password, allowInsecureTls }` | `Result<RemoteSource, IpcError>` |
  | `remote_source_update` | `{ id, patch }` | `Result<RemoteSource, IpcError>` |
  | `remote_source_remove` | `{ id }` | `Result<(), IpcError>` |
  | `remote_source_list` | `{}` | `Result<Vec<RemoteSource>, IpcError>` |
  | `remote_source_test_connection` | `{ id }` | `Result<ConnectionStatus, IpcError>` — pings, records the result, reports which auth mode worked |
  | `remote_browse_artists` | `{ sourceId }` | `Result<Vec<ArtistID3>, IpcError>` |
  | `remote_browse_artist` | `{ sourceId, artistId }` | `Result<ArtistDetail, IpcError>` |
  | `remote_browse_album` | `{ sourceId, albumId }` | `Result<AlbumDetail, IpcError>` |
  | `remote_search` | `{ sourceId, query }` | `Result<SearchResult3, IpcError>` |
  | `remote_play` | `{ sourceId, remoteId }` | `Result<(), IpcError>` — resolves a `MediaSource::Url` via `stream_url()` and calls `Player::load_and_play` with a synthetic `PlaybackSource::Remote` id, bypassing `tracks`/`track_id` entirely |
  | `remote_stream_url` / `remote_cover_art_url` | `{ sourceId, remoteId, size? }` | `Result<String, IpcError>` — pure builders (no network), used by the frontend for `<img>`/direct playback URLs |

  `AppState` gains a small in-memory registry — `Arc<RwLock<HashMap<i64, SubsonicClient>>>` keyed by `remote_sources.id` — so a `SubsonicClient` (which owns a `reqwest::Client` with per-source TLS settings, §2.4) is built once per source and invalidated on `remote_source_update`/`remote_source_remove`, rather than reconstructed on every call.
- `remote_play`/queueing remote tracks deliberately get **new, separate commands** rather than widening `player_play`/`queue_add`'s existing `{ trackId: i64 }` signature to a `PlaybackSource` union. Changing those signatures is the architecturally "correct" long-term shape (one command, one source-agnostic argument), but it touches every existing call site for a milestone whose goal is additive read/stream support — a unification pass is reasonable follow-up once the pattern (and whether persisted mixed queues are wanted at all, §2.3) has proven out.

**Implementation note:** `remote_play` calls the player with a synthetic track id of `-1` — negative, so it can never collide with a real `tracks.id` (`AUTOINCREMENT`, always positive) — since a remote track has no `tracks` row for the player's existing `track_id: i64` parameter to reference. Phase 4's now-playing lookup will need to recognize this sentinel and resolve the actual remote track's identity from `PlaybackSource::Remote` rather than treating it as a local track id.

**Shippable state:** the whole feature is exercisable via `tauri dev` console / a CLI harness without any UI — add a source, list its artists, play a track, hear it. This is a real milestone checkpoint even before Phase 4 lands.

### Phase 4 — UI

- Settings pane: remote sources list (name, base URL, connection badge sourced from `last_ping_ok`), add/edit/remove forms, an "allow self-signed certificate" toggle wired to `allowInsecureTls` (§5).
- Remote browse views: an artist-list / album-grid / album-detail route tree parallel to the existing local-library views (`library.rs`'s command surface and its UI consumers), scoped under a per-source route (e.g. a sidebar entry per configured source), backed by `remote_browse_*` and cached via TanStack Query keyed on `(sourceId, endpoint, params)` — no `signal-db`/local cache involved, consistent with the no-catalog-sync non-goal.
- Queue integration: queue rows render a source badge/icon for `PlaybackSource::Remote` entries; "play"/"queue" actions on a remote track call `remote_play`/the runtime-only remote queue path (§2.3) instead of the local `queue_add`. **Implementation note:** this didn't ship as described. `queue_items.track_id` is `INTEGER NOT NULL REFERENCES tracks(id)`, and remote tracks deliberately have no `tracks` row, so a remote song cannot be staged in the DB-backed queue at all. What shipped instead: remote albums play through the *play context* (the implicit follow-on order), which delivers auto-advance, gapless, shuffle and repeat for remote albums, but no visible queue rows and no mixed local/remote staging. Staging remote tracks would require a schema change and is not in v1; the settings pane states this limitation to the user.
- Cover art: `remote_cover_art_url` returns a fully authenticated URL (query-string token, same as `stream_url`) used directly as an `<img src>` — no `signal-art://`-style custom protocol handler needed, since the credentials travel in the URL itself rather than through a Tauri-mediated fetch. This needs `tauri.conf.json`'s CSP `img-src` relaxed to allow arbitrary remote hosts (sources are user-added, so a static allowlist can't work); `<img>` tags can't execute script, so this is a materially smaller CSP concession than relaxing `connect-src`/`script-src` would be. If that's judged too permissive, the fallback is a `remote_cover_art_fetch` command that proxies bytes through Rust and returns them as a data URL — more consistent with the local `signal-art://` model but adds a round trip and loses browser-level HTTP caching; noted as an open question (§6). **Implementation note:** neither horn of that tradeoff was needed. `src-tauri/tauri.conf.json` already sets `"csp": null`, so remote cover art loads through a plain `<img src>` with no config change and no proxy command. Cover URLs are still built in Rust — they carry the auth token — but reach the UI through a cached TanStack Query rather than being derived inline the way local `artworkUrl` is.

**Shippable state:** the full user-facing feature — add a server, browse it, play from it. (Queue visibility didn't ship as scoped here — see the inline note above.)

### Phase 5 — Polish

- **Network failure mid-playback.** `engine.rs`'s `Event::EndFile(reason) if *reason == mpv_end_file_reason::Error` already fires for any playback failure, local or remote, and currently produces a generic `tracing::warn!("playback ended with error")` plus a stopped state — no user-facing distinction today. Extend this path (or the `src-tauri` bridge layer, which already knows the `PlaybackSource` of whatever was just loaded) so a dropped remote connection surfaces a specific "connection to `<source name>` lost" toast rather than the generic playback-failed message, per the existing toast/log-line split in `01-architecture.md`'s error handling section.
- **Gapless-with-remote testing.** `SetNext`'s prefetch (mpv's `prefetch-playlist yes`, set in `init_mpv`) now has to complete over the network before the current track ends, not just read from local disk. Verify against a real remote server (or an artificially throttled loopback) whether the existing prefetch trigger point gives mpv enough lead time, or whether `src-tauri` needs to call `SetNext` earlier (bigger lookahead) specifically for `PlaybackSource::Remote` upcoming tracks. This can't be meaningfully validated on `localhost` alone (§5, Risks) — it needs a real network hop.
- **Scrobbling remote plays — corrected from the original assumption.** The existing `PlaySource::Remote` value on `play_events.source` (migration `0005_remote_play_source.sql`) means something different from what this feature needs: it's set by `signal-server`'s own `annotation.rs::scrobble` handler, when a *phone connects to Signal's embedded server* and reports a play of a **local** track (`play_events.track_id NOT NULL REFERENCES tracks(id)` — the row always exists). That's the inbound direction. This feature is outbound: Signal, as a client, streaming a track that has no `tracks` row at all. Reusing the literal `'remote'` `play_events` value for that is not just a naming collision, it's schema-incompatible — there's no `track_id` to satisfy the `NOT NULL REFERENCES` constraint for a track that was never scanned. The correct v1 behavior, and a simpler one: don't write to local `play_events` for remote plays at all. Instead, call the **remote server's own** `scrobble` endpoint (`signal_subsonic_client`, once added to its method set) with the remote track's id once playback crosses the same "counted as played" threshold `signal-player`/`src-tauri` already use for local plays — so the *remote* server's play counts and "now playing" state update, which is what a well-behaved Subsonic client is expected to do, without touching Signal's local stats schema at all. Unifying local and remote listening history into one stats view is real follow-up work (nullable `track_id` + a `remote_source_id`/`remote_id` pair on `play_events`, and adapting `StatsRepo`'s codec/genre-joining queries to tolerate remote rows that have neither) — explicitly out of scope here and listed in §6.

## 4. Database changes

One migration, `migrations/0007_remote_sources.sql`, following the existing conventions (`03-database-schema.md` §1: `INTEGER PRIMARY KEY AUTOINCREMENT`, ISO-8601 `TEXT` timestamps via `strftime`, booleans as `INTEGER CHECK (col IN (0,1))`):

```sql
CREATE TABLE remote_sources (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    base_url            TEXT NOT NULL,
    username            TEXT NOT NULL,
    password            TEXT NOT NULL,
    auth_mode           TEXT NOT NULL DEFAULT 'token' CHECK (auth_mode IN ('token', 'legacy_p')),
    allow_insecure_tls  INTEGER NOT NULL DEFAULT 0 CHECK (allow_insecure_tls IN (0, 1)),
    enabled             INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    last_ping_at        TEXT,
    last_ping_ok        INTEGER CHECK (last_ping_ok IN (0, 1)),
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_remote_sources_name ON remote_sources(name);
-- sidebar entries are keyed by name; prevents two sources confusingly sharing a label
```

`auth_mode` persists which scheme actually worked for this server (§2.4/§5 — token-first with a `p=` fallback probe on `remote_source_test_connection`), so steady-state requests don't re-probe every time. `password` is plaintext, matching the existing `server.password` precedent (§2.4). Nothing changes on `tracks`, `queue_items`, or `play_events` in v1 — see §2.3 and §3, Phase 5 for why both of those were considered and deliberately deferred rather than touched.

## 5. Risks

| # | Risk | Mitigation |
|---|------|------------|
| 1 | **Gapless prefetch over the network.** `SetNext`'s mpv-side prefetch assumes the next file loads fast enough to be ready before the current one ends; that assumption holds trivially for local disk and not necessarily for a remote HTTP stream on a slow link. | Treat prefetch failure as non-fatal — fall back to a small gap rather than blocking/erroring; for `PlaybackSource::Remote` upcoming tracks, trigger `SetNext` earlier (larger lead time) than the local-file trigger point (Phase 5). Must be tested against a real remote server or a throttled connection — `localhost` round-trips are too fast to surface this. |
| 2 | **Mixed local+remote queue persistence.** `queue_items` schema (§2.3) can't represent a remote entry without real surgery. | v1 keeps remote queue entries runtime-only (not persisted); explicit, visible tradeoff rather than a silent gap — revisit if there's demand once the read/stream path has shipped. |
| 3 | **Legacy servers that only accept `p=`.** Some older or restrictively-configured Subsonic servers reject/mishandle token (`t`+`s`) auth. | `remote_source_test_connection` probes token auth first; on failure, retries once with `p=` and persists whichever worked in `remote_sources.auth_mode` so steady-state requests don't re-probe. |
| 4 | **Self-signed/private-CA TLS certs.** The workspace's `reqwest` dependency is `rustls-tls` only (`Cargo.toml`), no native OS trust store integration, and rustls rejects self-signed certs outright by default — common on homelab Navidrome/Airsonic instances, exactly this feature's likely audience. | Per-source `allow_insecure_tls` toggle (§4) mapped to a distinct `reqwest::Client` built with `danger_accept_invalid_certs(true)` for *that source only*, never a global setting; UI shows a persistent warning badge when enabled. Consider also enabling `rustls-tls-native-roots` so legitimately CA-signed internal certs (private CA, not self-signed) work without the insecure toggle at all. |
| 5 | **Bundled libmpv may lack network-capable protocols.** The roadmap's own risk register (`07-roadmap.md` #1) already flags that macOS/Windows vendor a pinned, size-conscious libmpv/ffmpeg build; a build trimmed for size can drop `https`/TLS-enabled network protocol support, which this feature is the first to actually exercise (today's app never asks mpv to open a URL). | Verify explicitly, early (Phase 2's integration test is the first real signal) that the vendored build opens an `https://` stream, not just `http://` loopback — a loopback-only test would pass even on a build that's missing TLS support. **Status: partially retired.** Phase 2/3 tests confirm mpv opens and plays `http://` loopback streams on the macOS dev build. The `https://` case — the one that actually matters in production, since real remote servers are typically TLS — remains unverified; closing it out needs the real-server manual smoke test (§7). **Phase 4 update:** remote-to-remote gapless advance — two separate loopback servers, crossing connections the way consecutive tracks of a remote album do — is now proven working on the macOS dev build (`crates/signal-player/tests/url_playback.rs`), retiring this risk further; `https://` remains unverified. |
| 6 | **Third-party server response variance.** Navidrome/Airsonic/Gonic/etc. don't agree on which optional OpenSubsonic fields they populate, or occasionally return non-JSON (HTML error pages) on failure. | Shared DTOs default-tolerant per §2.2; Phase 1 test fixtures include real third-party captures, not just Signal's own shape; `ClientError::Parse` surfaces malformed/unexpected bodies as a clear error rather than a panic. |
| 7 | **Remote-source credentials are a higher-value target than Signal's own LAN password.** Stored plaintext (§2.4), same mechanism as the existing `server.password`, but now for *other services'* accounts, which users more often reuse elsewhere. | Documented tradeoff, not silently accepted; OS keyring recommended as a stretch goal, not a v1 blocker. |
| 8 | **Cover-art CSP relaxation.** Arbitrary user-added hosts for `<img src>` means `img-src` can't be a static allowlist. | `<img>` can't execute script, so this is a narrow, low-severity relaxation; documented explicitly as a deliberate choice (§3, Phase 4) with a proxy-based fallback noted as an alternative if judged too permissive. |

## 6. Open questions

- Should remote queue entries eventually persist across restarts (schema change to `queue_items`, §2.3), or is runtime-only durable enough for how people actually use this?
- Is any write-back beyond scrobbling (star/rate a remote track, edit a remote playlist) ever in scope, or does this stay strictly read+stream+scrobble indefinitely?
- Is a unified local+remote stats/listening-history view worth the `play_events` schema work described in §3 Phase 5, or does remote listening simply not show up in Signal's own stats?
- Does the OS keyring stretch goal (§2.4, §5) need to land before this ships, given it's *other services'* credentials, not just Signal's own LAN password?
- Worth a `remote_cover_art_fetch` proxy command instead of relaxing `img-src` (§3 Phase 4), trading a CSP concession for an extra IPC round trip and no browser HTTP caching? **Resolved in Phase 4:** moot — `src-tauri/tauri.conf.json` already sets `"csp": null`, so cover art loads via a plain `<img src>` with no config change and no proxy command needed (§3, Phase 4).
- Any real-world server that needs `maxBitRate`/`format` hints on `stream` to be playable at all by mpv (the non-goal in §1 assumes raw stream is always fine) — revisit if that assumption breaks in practice.

## 7. Testing strategy

The single most useful fact about this feature: **Signal already has a fully working OpenSubsonic server embedded in the same repo**, and `signal-server::start()` binds an ephemeral port (`port: 0`) specifically so tests can spin up a real instance in-process with no mocking. Pointing `signal-subsonic-client` at `signal_server::start(db, ServerConfig { port: 0, .. })` in a test gives a hermetic, fast, dependency-free integration test that exercises **both** crates' understanding of the wire format against each other in one shot — if `signal-server`'s DTOs and `signal-subsonic-client`'s parsing ever drift, this test catches it immediately, with no wiremock/fixture-maintenance burden for the happy path. This is the backbone of the test plan, not a nice-to-have:

- **Phase 1 (`signal-subsonic-client` crate tests):**
  - Unit: DTO deserialization against canned JSON — Signal's own shape (trivial, cross-checked against `signal-server`'s existing `dto.rs` tests) plus at least one captured real Navidrome and one real Airsonic/Gonic response, to catch the field-completeness variance §2.2/§5 #6 calls out. **Implementation note:** shipped as three hand-authored fixtures (Navidrome `getAlbum`, Gonic `getArtists`, Airsonic `search3`), built from each server's documented and observed response shape, not captured from a live instance. They pin parsing behavior against a plausible shape, but are not evidence any specific server build answers exactly this — the manual smoke test below is what actually closes that gap, and it hasn't been run yet.
  - Unit: auth token generation cross-validated byte-for-byte against `signal-server::auth`'s own test vectors (§2.4) — same `(password, salt)` in, same hex token out, on both sides of the wire.
  - Integration: real embedded `signal-server` on an ephemeral port, full read path — `ping`, `getArtists`, `getArtist`, `getAlbum`, `search3`, a ranged `stream` request against a `fixtures/` audio file, `getCoverArt`. This is where wiremock/hand-rolled mock servers are explicitly **not** needed for the happy path.
  - Wiremock (or a minimal standalone axum test server) reserved for what Signal's own server will never naturally produce, since it's internally consistent by construction: the legacy `p=`-fallback probe (§5 #3), self-signed cert accept/reject behavior (§5 #4), malformed/non-JSON error bodies (§5 #6), and Subsonic error-code edge cases beyond what `signal-server` itself returns.
- **Phase 2 (`signal-player`):** the same "app tests against itself" trick — a `fixtures/` audio file served over loopback HTTP (via a throwaway embedded server or a minimal static file server), `Player::load_and_play(id, MediaSource::Url(..))`, assert state reaches `Playing` with a non-zero `duration_ms`.
- **Phase 3/4:** IPC command handler tests following the existing pattern (per-command `tracing` span, `Result<T, SignalError>`); UI covered by the frontend hygiene bar already in the roadmap's Definition of Done (TypeScript strict, no uncaught console errors).
- **Manual smoke test addition:** unlike the self-test above, real-world server quirks (§5 #6) can only be caught against a real third-party server. Before M6 is called done, configure a remote source against an actual Navidrome (or similar) instance — a Docker container is a five-minute setup — and run the full browse-and-play flow at least once, per-OS, alongside the existing M2/M4-style manual smoke requirement in `07-roadmap.md`'s Definition of Done. **Status: not yet performed.** Phases 1–3 have shipped without it; it's still required before M6 as a whole can be called done, and is also what's needed to close out Risk 5's `https://` gap (§5).

## 8. Rough estimates

Using the roadmap's own sizing (`07-roadmap.md`): **S** = a few days, **M** = one to two weeks, **L** = three-plus weeks, for a single focused contributor.

| Phase | Scope | Size |
|---|---|---|
| 1 | `signal-subsonic-types` + `signal-subsonic-client` crates, DTO extraction, auth, unit + self-test integration tests | M |
| 2 | `MediaSource` plumbing through `signal-core`/`signal-player`, URL playback integration test | S |
| 3 | Migration, `RemoteSourceRepo`, `SignalError` wiring, full `remote_*` IPC command set, per-source client registry in `AppState` | M |
| 4 | Settings pane, remote browse route tree, queue badge/integration, cover art wiring + CSP change | L |
| 5 | Failure-path UX, gapless-with-remote validation against a real server, corrected scrobble-to-remote behavior | S–M |

**M6 overall: L** — dominated by Phase 4's new UI surface (a second browse tree parallel to the existing library views) and the real-server validation work in Phase 5 that can't be shortcut by the self-test trick.
