# Plugin Architecture

Signal's core stays deliberately small: local playback, a local library, and local search, with nothing that talks to the network unless the user explicitly turns it on. Everything that touches the outside world — scrobbling, lyrics fetching, Discord presence, home automation hooks — lives in the plugin system instead of the core crates. This document describes that system: the trait plugins implement, the two-tier native/external design, the event surface plugins consume, and concrete sketches for the first three plugins Signal ships.

## Philosophy

Signal has a no-telemetry principle: nothing leaves the machine unless the user asked for it. That principle only holds if "network-touching" and "core" are structurally separated, not just conventionally separated. So the rule is simple:

- **If a feature makes a network request, it is a plugin, full stop.** Scrobblers, lyrics providers, Discord Rich Presence, ListenBrainz, Home Assistant integration — none of these belong in `signal-core`, `signal-db`, `signal-player`, or any other core crate, even if the feature feels central to the product experience.
- **Plugins are opt-in and off by default.** Installing or building Signal with a plugin available does not mean it's active. A plugin makes its first network call only after a user has explicitly enabled it and (where relevant) authenticated it.
- **The core has no knowledge of what a specific plugin does.** Core crates expose a generic event bus and a generic plugin host; they don't import `signal-plugins` or know that Last.fm exists. This keeps the dependency graph one-directional — plugins depend on core, core never depends on plugins — and keeps the core testable and auditable without wading through integration-specific code.

## Two-Tier Design

Plugins come in two tiers, introduced at different points in the roadmap.

### Tier 1: Native Rust plugins (MVP, M5)

Tier 1 plugins are Rust trait objects compiled directly into the `signal-plugins` crate, each gated behind its own Cargo feature flag (`mpris`, `lastfm`, and so on). They run in-process, share the same address space and event bus as the rest of the app, and are built and shipped as part of the same binary.

This is the only tier that exists at MVP (M5 in the [roadmap](./07-roadmap.md)) and it's what MPRIS and Last.fm are built as. Being in-process means zero IPC overhead and full access to Rust's type system for the plugin API, at the cost of every tier-1 plugin needing to be reviewed and merged into the main repository like any other code.

### Tier 2: External plugins (post-MVP)

Tier 2 plugins are separate OS processes speaking a stable JSON-RPC-over-stdio protocol to the Signal host. The host spawns the plugin's executable, sends it framed JSON-RPC requests over stdin, and reads framed JSON-RPC responses and notifications over stdout. This is the same shape as the Language Server Protocol, deliberately — it's a well-understood, boring, battle-tested pattern for exactly this kind of "host talks to arbitrary third-party executable" problem.

External plugins let anyone write a plugin in any language capable of reading stdin and writing stdout — Python, Node, Go, another Rust binary compiled independently — without needing to build against Signal's internal Rust types or even use the same Rust compiler version Signal was built with.

**Why not dynamic libraries (`.dylib`/`.dll`/`.so`) instead?** Rust has no stable ABI across compiler versions, and even within the same compiler version, struct layout, generic monomorphization, and trait object vtables are not guaranteed stable across crate versions. A `dylib`-based plugin would need to be recompiled against the exact `signal-core` and Rust compiler version the host binary was built with, which turns every Signal release into a breaking change for every third-party plugin. `extern "C"` FFI boundaries could work around this, but at that point you've reinvented a worse, unsafe version of the JSON-RPC-over-stdio approach with none of the process isolation benefits and all of the memory-safety risk of loading arbitrary native code into the host's address space.

**Why not WebAssembly for tier 2, at least initially?** WASM (via `wasmtime` or similar) is attractive for sandboxing, but the plugins Signal actually needs — MPRIS's D-Bus access, Discord's local IPC socket, eventually things like Home Assistant on the local network — need OS-level resources that WASI either doesn't expose or exposes through host-function shims that would need to be designed, secured, and maintained one capability at a time before any plugin could do useful work. That's a lot of sandboxing infrastructure to build before the first external plugin can ship anything. Stdio JSON-RPC gets external plugins working with a fraction of the effort, using OS process boundaries (which are well understood and already relied on everywhere) as the isolation mechanism instead.

WASM stays on the table as a **future sandboxed alternative for tier 2**, once the host-function surface for things like scoped network access and scoped filesystem access has stabilized enough to be worth building. It would let untrusted plugins run with real capability restrictions enforced by the runtime rather than by developer discipline. That's a deliberate later step, not a rejected idea.

### Sketch of the tier-2 protocol

Not finalized, but the shape is settled enough to plan around. Framing follows the same `Content-Length` header approach LSP uses over stdio (a length-prefixed line followed by a raw JSON body), which sidesteps having to invent yet another message-framing scheme:

```
Content-Length: 73

{"jsonrpc":"2.0","method":"signal/trackEnded","params":{"trackId":"..."}}
```

Host-to-plugin notifications mirror the `SignalEvent` surface plugins already consume in tier 1 (`signal/trackStarted`, `signal/trackEnded`, `signal/stateChanged`, `signal/queueChanged`). Plugin-to-host requests are capability-gated RPCs, for example:

- `db/getTrack { trackId }` → track metadata, permitted only if the plugin declared `db-read`
- `http/request { method, url, headers, body }` → proxied outbound HTTP call, permitted only if the plugin declared `network`, and only to hosts the plugin declared it needs
- `config/get { key }` / `config/set { key, value }` → namespaced config storage, same `plugin.<name>.*` scoping as tier 1
- `commands/register { id, title }` → registers a palette command; invocations come back to the plugin as a `signal/commandInvoked { id }` notification

On connect, the plugin sends an `initialize` request declaring its name, version, and requested capabilities, and the host responds with which of those were actually granted (based on the manifest the user approved at install time) — so a plugin can detect at startup that, say, `network` was denied and disable its own scrobble-submission logic rather than failing opaquely on the first blocked request.

## The `SignalPlugin` Trait

Tier-1 plugins implement one trait:

```rust
pub trait SignalPlugin: Send + Sync {
    /// Stable, lowercase identifier used in config keys, palette
    /// commands, and log lines (e.g. "lastfm", "mpris", "discord").
    fn name(&self) -> &str;

    /// Plugin version, independent of Signal's own version.
    fn version(&self) -> &str;

    /// Called once when the plugin is enabled. Plugins do setup
    /// here (spawn tasks, open sockets, load stored config) and
    /// return an error if they can't initialize — a failed
    /// on_load disables the plugin and logs why, it never panics
    /// the host.
    fn on_load(&mut self, ctx: PluginContext) -> anyhow::Result<()>;

    /// Called for every SignalEvent published on the bus while
    /// this plugin is enabled. Plugins filter for the events they
    /// care about and ignore the rest.
    fn on_event(&mut self, event: &SignalEvent);

    /// Called when the plugin is disabled or the app is shutting
    /// down. Plugins flush any pending state here (e.g. an offline
    /// scrobble queue) on a best-effort basis.
    fn shutdown(&mut self);
}
```

`PluginContext` is what `on_load` receives, and it's the entire surface area a plugin has into the rest of Signal:

- **`db: Arc<dyn LibraryReader>`** — a read-only view over `signal-db`. Plugins can look up track/album/artist metadata to enrich an outbound API call (e.g. Last.fm wants artist + track + album + duration), but cannot write to the library, queue, or playback state directly. Any state change a plugin wants (skip a track, add a palette command) goes through an explicit part of `PluginContext`, not a raw DB handle.
- **`events: EventSubscription`** — a subscription handle to the `SignalEvent` broadcast bus. This is what feeds `on_event`; it's set up automatically by the host and plugins don't manage it directly, but it's part of the context conceptually.
- **`config: PluginConfig`** — namespaced key-value storage backed by the settings table, scoped to `plugin.<name>.*` so plugins can't read or clobber each other's config or core settings. This is where auth tokens, offline queues, and user preferences for the plugin live.
- **`http: HttpAllowance`** — the only way a tier-1 plugin is permitted to make outbound network calls. It's a pre-configured `reqwest` client wrapper, not raw socket access, which keeps all outbound plugin traffic visible to the host for logging and (later) rate limiting.
- **`commands: CommandRegistrar`** — lets a plugin register entries in the command palette (e.g. Last.fm's plugin registers a "Last.fm: reauthenticate" command). Registered commands are namespaced by plugin name so palette entries are traceable to their source.

| Field | Type | Read/write | Notes |
|---|---|---|---|
| `db` | `Arc<dyn LibraryReader>` | read-only | Track/album/artist lookups only; no queue or playback mutation |
| `events` | `EventSubscription` | subscribe-only | Feeds `on_event`; managed by the host |
| `config` | `PluginConfig` | read/write | Namespaced to `plugin.<name>.*`, persisted in the settings table |
| `http` | `HttpAllowance` | outbound only | Wrapped `reqwest` client; every call is loggable by the host |
| `commands` | `CommandRegistrar` | write-only (register) | Palette entries namespaced by plugin name |

Notably absent: no direct filesystem access, no raw socket access, and no write path into `signal-db`. Anything a plugin needs beyond this table is a sign the context needs a new, narrowly-scoped field — not a reason to hand out a broader handle. (The one deliberate exception is the narrow playback-control handle MPRIS needs, described in its sketch below, since remote-control surfaces are the one class of plugin that must be able to drive playback rather than just observe it.)

## Event Surface

Plugins consume events from the same `SignalEvent` bus defined in `signal-core`. The events most relevant to plugins:

- **`TrackStarted { track_id }`** — fired the moment playback begins on a track.
- **`TrackEnded { track_id, ms_played, completed, skipped }`** — fired when a track stops being the active track, whether because it finished, the user skipped it, or playback was stopped. `ms_played` is how long it was actually audible; `completed` is true if it played to the end; `skipped` is true if the user explicitly advanced past it.
- **`StateChanged { state }`** — playing/paused/stopped transitions, independent of which track is active.
- **`QueueChanged`** — the queue was modified (add/remove/reorder). No payload beyond the fact that it changed; plugins that need queue contents call back through `PluginContext::db` or a queue reader if they need to inspect it.

### Walkthrough: the Last.fm scrobbler

This is the canonical example of why `TrackEnded` carries `ms_played`/`completed`/`skipped` instead of just being a bare "track changed" notification. Last.fm's scrobble API has a real-world rule attached to it: a track only counts as "scrobbled" if the user listened to at least half of it, or at least 4 minutes, whichever comes first (and the track itself has to be longer than 30 seconds to be eligible at all).

Here's the flow end to end:

1. `TrackStarted` fires. The plugin looks up the track's duration via `ctx.db` and records `(track_id, started_at)` in memory. It also fires a "now playing" update to Last.fm's `track.updateNowPlaying` endpoint — this is a courtesy call, not the actual scrobble, and failures here are silently ignored.
2. The track plays. The plugin does nothing else until it ends — it does not poll `player:progress`; `ms_played` on `TrackEnded` is authoritative and computed by `signal-player`, not reconstructed by the plugin from a stream of progress ticks.
3. `TrackEnded` fires with `ms_played` and `completed`/`skipped`. The plugin computes: is `ms_played >= min(duration_ms / 2, 240_000)` and is `duration_ms > 30_000`? If yes, this is a real scrobble.
4. A real scrobble is appended to an **offline queue** stored in `ctx.config` under `plugin.lastfm.pending_scrobbles` — not submitted synchronously inline in the event handler, because `on_event` handlers have a timeout (see Lifecycle below) and a flaky network call must never risk that.
5. A separate background task owned by the plugin (spawned in `on_load`) drains the offline queue periodically, submitting batches to Last.fm's `track.scrobble` endpoint via `ctx.http`, with retry/backoff on failure. A scrobble that fails to submit stays queued indefinitely rather than being dropped — this matters for laptop users who scrobble on a flight and sync once they're back online.
6. On `shutdown`, the plugin does one last best-effort flush attempt but does not block indefinitely waiting on it.

Everything else — auth, palette commands, UI — sits on top of this core flow; see the concrete sketch below for the rest.

## Lifecycle and Failure Isolation

Plugins are third-party code (even tier-1 plugins living in the main repo are logically separable, and tier-2 plugins are fully untrusted external processes), so the host is built assuming any plugin can misbehave:

- **Each plugin runs on its own Tokio task(s).** `on_event` for a given plugin never blocks or delays event delivery to other plugins or to the core; the host dispatches to each plugin's task independently rather than calling `on_event` inline on the bus's delivery path.
- **Panics are caught, not propagated.** Plugin task bodies wrap plugin calls in `catch_unwind` (or rely on the fact that a panic inside a spawned Tokio task only fails that task's `JoinHandle`, not the whole process). A panicking plugin is caught, immediately disabled, and a `log:line` event is emitted at `error` level naming the plugin and the panic message. The rest of Signal — playback, the UI, other plugins — is unaffected.
- **Handlers are timeout-bound.** Every call into `on_load`, `on_event`, and `shutdown` is wrapped in a timeout (a few seconds for `on_event`, longer for `on_load` since it may do first-time setup like a network handshake). A handler that hangs past its timeout is treated the same as a panic: the plugin is disabled and logged, not left to block the event bus indefinitely.
- **A disabled plugin stays disabled until the user re-enables it.** The host does not silently retry a plugin that just crashed — that risks a crash loop. Re-enabling is a deliberate user action from the settings UI or the command palette, at which point `on_load` runs fresh.

For tier-2 external plugins, the same principles apply one level up: the child process itself is the isolation boundary. A hung or crashed child process is detected via the JSON-RPC transport (broken pipe, timeout on a request) and handled exactly like a tier-1 timeout/panic — disable, log, wait for the user.

## Config and Enable UX

Plugins are surfaced in two places:

- **Settings UI**, in a dedicated Plugins section: a list of available plugins (tier 1 plugins bundled in the current build, tier 2 plugins the user has installed), each with an enable/disable toggle and a per-plugin configuration panel rendered from that plugin's declared config schema (auth fields, throttle intervals, whatever the plugin needs).
- **Command palette**, for keyboard-first control matching the rest of the app:
  - `plugin list` — shows enabled/disabled state for every known plugin
  - `plugin enable <name>` — enables a plugin, running `on_load`
  - `plugin disable <name>` — disables a plugin, running `shutdown`

Both surfaces go through the same host-level enable/disable path — the settings UI toggle and the palette command are two entry points into identical logic, not two separate code paths that can drift.

## Concrete Plugin Sketches

### MPRIS (Linux)

Exposes Signal as a standard `org.mpris.MediaPlayer2` D-Bus service so Linux desktop environments (GNOME, KDE, and anything else that speaks MPRIS) can show now-playing info and route media keys to Signal without any Signal-specific integration on their end.

- Built on `zbus` for the D-Bus interface implementation.
- **`SignalEvent` → MPRIS direction:** `TrackStarted`/`StateChanged` update the `PlaybackStatus` and `Metadata` (title, artist, album, art URL — pulled via `ctx.db`) properties exposed over D-Bus and emit the corresponding `PropertiesChanged` signal so desktop widgets update live. `player:progress` events update the `Position` property (read on demand per the MPRIS spec, not pushed).
- **MPRIS → Signal direction:** incoming D-Bus method calls (`Play`, `Pause`, `PlayPause`, `Next`, `Previous`, `Seek`, `Stop`) are translated into the same IPC-equivalent calls the frontend uses (`player_play`, `player_pause`, etc.) via an internal handle the plugin gets at `on_load` — this is the one case where a plugin needs to *drive* playback rather than just observe it, so the plugin context includes a narrow playback-control handle in addition to the read-only DB view.
- No network access, no config beyond "enabled." Ships enabled by default on Linux builds since it has zero external-world footprint and is expected desktop behavior, not an opt-in integration in the no-telemetry sense.

### Last.fm

Full scrobbler as walked through above, plus the parts not covered there:

- **Auth flow:** Last.fm uses a web-auth handshake — the plugin requests a token from Last.fm's API, opens the user's default browser to a Last.fm authorization page carrying that token, and polls Last.fm's `auth.getSession` endpoint until the user approves in-browser. The resulting session key is written to `ctx.config` under `plugin.lastfm.session_key`. Nothing here uses an embedded webview; it's the system browser, kept explicit so the user can see exactly what domain they're authenticating against.
- **Offline queue** lives in `ctx.config` under `plugin.lastfm.pending_scrobbles` as a JSON array capped at a reasonable size (Last.fm's own API caps batch submission size too), so a laptop that's offline for days doesn't grow the queue unboundedly — oldest entries are dropped past the cap rather than blocking new scrobbles from being queued.
- **Palette commands:** `plugin enable lastfm` triggers the auth flow if no session key is stored yet; a dedicated `lastfm: reauthenticate` command (registered via `ctx.commands`) lets the user re-run auth without disabling/re-enabling the whole plugin.

### Discord Rich Presence

Shows the currently playing track in the user's Discord profile status.

- Connects to the local Discord client over its IPC mechanism — a Unix domain socket on macOS/Linux (`discord-ipc-0` in the appropriate runtime directory) or a named pipe on Windows. This is entirely local; no network request leaves the machine, Discord's own client handles broadcasting the presence to Discord's servers.
- **`SignalEvent` → Discord direction:** `TrackStarted` and `StateChanged` trigger a presence update (track title, artist, album art if a public art URL is available, elapsed time). `TrackEnded` with no immediate next track clears the presence rather than leaving stale "now playing" data visible.
- **Throttling:** Discord's IPC rate-limits presence updates (informally, updates faster than about once every 15 seconds tend to get dropped or cause the client to ignore subsequent ones). The plugin coalesces rapid-fire updates — e.g. quick track skips — into a single update per throttle window rather than firing one per event, and truncates title/artist strings to Discord's field length limits before sending.
- If the Discord client isn't running, socket connection fails; the plugin treats this as a normal non-error state (retry connecting periodically in the background) rather than disabling itself, since "Discord isn't open right now" isn't a plugin failure.

## Security and Trust Model

The two tiers carry different trust levels, and the host treats them differently:

- **Tier 1 (native, compiled-in) is trusted.** These plugins ship as part of the Signal binary, went through the same PR review as any other core change, and run with full `PluginContext` access as described above. A user "trusts" tier-1 plugins simply by trusting the Signal build they downloaded — there's no separate approval step, but every tier-1 plugin is still off by default and the user opts in per-plugin.
- **Tier 2 (external processes) is untrusted by default.** Each external plugin ships with a manifest declaring the capabilities it needs — for example `network` (permission to make outbound HTTP calls, proxied by the host rather than the plugin having raw socket access), `db-read` (permission to query the read-only library view), and so on. When a user installs an external plugin, the host reads the manifest and presents the requested capabilities for explicit approval before the plugin is allowed to run at all — the same shape as a mobile app permission prompt. A plugin cannot silently request `network` after the fact; a manifest change requires re-approval.
- **Process isolation backs the capability model for tier 2.** Because external plugins only communicate over the stdio JSON-RPC channel, a capability the user didn't grant isn't just policy-enforced, it's structurally unavailable — an external plugin has no file descriptor, socket, or DB connection of its own; every capability it uses is a request the host receives, checks against the approved manifest, and either fulfills or rejects. This is what makes the "declared capabilities" model meaningful rather than an honor system.

This two-tier trust split is also why tier 2 didn't ship at MVP: building the manifest format, the approval UI, and the capability-enforcement plumbing in the host is real work, and it wasn't worth doing before there was a proven need for plugins beyond the ones the core team ships itself.
