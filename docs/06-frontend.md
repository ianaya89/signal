# Frontend Architecture

This document describes the React/TypeScript frontend that renders inside Signal's Tauri WebView: directory layout, routing, the state-ownership split between TanStack Query and Zustand, the event bridge that keeps them fed, the component breakdown for the three-pane layout, and the performance rules that keep a 100k-track library scrolling smoothly.

## 1. Directory tree

```
src/
├── main.tsx                       # entry point: router + query client + event bridge bootstrap
├── routes/                        # TanStack Router file-based routes (CENTER pane only)
│   ├── __root.tsx                 # root layout: mounts AppShell, renders <Outlet/> into MainView
│   ├── index.tsx                  # redirects to /albums
│   ├── albums/
│   │   ├── index.tsx
│   │   └── $albumId.tsx
│   ├── artists/
│   │   ├── index.tsx
│   │   └── $artistId.tsx
│   ├── playlists/
│   │   └── $playlistId.tsx
│   ├── search.tsx
│   ├── stats.tsx
│   ├── logs.tsx
│   └── settings.tsx
├── components/
│   ├── layout/                    # AppShell, LibraryNav, MainView, StatusBar
│   ├── library/                   # AlbumGrid, AlbumCard, ArtistList, FolderTree
│   ├── queue/                     # QueuePanel, QueueRow
│   ├── inspector/                 # InspectorPane, TechnicalDetails
│   ├── palette/                   # PaletteOverlay, CommandLine
│   ├── player/                    # PlaybackBar, SeekBar, VolumeControl
│   ├── logs/                      # LogViewer
│   ├── stats/                     # StatsHeatmap, StatsSummary
│   └── ui/                        # shadcn/ui primitives (button, dialog, command, ...)
├── lib/
│   ├── ipc/
│   │   ├── invoke.ts               # typed wrappers around Tauri invoke()
│   │   ├── types.ts                # DTOs mirroring signal-core (see docs/05-ipc-api.md)
│   │   └── events.ts               # single listen() subscription -> stores/query bridge
│   ├── keyboard/
│   │   ├── keymap.ts                # mode stack + pane-scoped binding registry
│   │   └── modes.ts                 # normal / palette / search / rename mode defs
│   └── search/
│       ├── parser.ts                # client-side query-language tokenizer/parser
│       └── commandRegistry.ts       # palette command list + fuzzy matcher
├── stores/
│   ├── playerStore.ts               # ephemeral PlayerState mirror, fed by events
│   ├── uiStore.ts                   # focused pane, palette open, selection
│   └── queueStore.ts                # queue snapshot, fed by queue:changed
└── hooks/
    ├── useAlbums.ts                 # useQuery wrappers, one per domain query
    ├── useKeymap.ts
    └── useNowPlaying.ts
```

Route modules only import from `routes/`, `components/layout`, and hooks — they never reach into `lib/ipc` directly for anything beyond the query/loader functions defined in `hooks/`.

## 2. Routing

TanStack Router's route tree renders exclusively into the CENTER pane. LEFT (`LibraryNav`), RIGHT (`InspectorPane` + `QueuePanel`), and BOTTOM (`CommandLine` + `StatusBar` + playback bar) are mounted once in the root layout and are never re-mounted on navigation.

```tsx
// src/routes/__root.tsx
import { createRootRoute, Outlet } from "@tanstack/react-router";
import { AppShell } from "@/components/layout/AppShell";
import { LibraryNav } from "@/components/layout/LibraryNav";
import { InspectorPane } from "@/components/inspector/InspectorPane";
import { QueuePanel } from "@/components/queue/QueuePanel";
import { StatusBar } from "@/components/layout/StatusBar";
import { CommandLine } from "@/components/palette/CommandLine";
import { PaletteOverlay } from "@/components/palette/PaletteOverlay";
import { PlaybackBar } from "@/components/player/PlaybackBar";

export const Route = createRootRoute({
  component: () => (
    <AppShell
      left={<LibraryNav />}
      center={<Outlet />}
      right={
        <>
          <InspectorPane />
          <QueuePanel />
        </>
      }
      bottom={
        <>
          <CommandLine />
          <StatusBar />
          <PlaybackBar />
        </>
      }
    >
      <PaletteOverlay />
    </AppShell>
  ),
});
```

Route tree:

```
/                        -> redirects to /albums (library/albums is the default view)
/albums                  -> AlbumGrid
/albums/$albumId         -> track list for one album, loader: libraryGetAlbum
/artists                 -> ArtistList
/artists/$artistId       -> albums by that artist
/playlists/$playlistId   -> TrackTable scoped to a playlist
/search                  -> TrackTable driven by the query-language parser
/stats                   -> StatsHeatmap + StatsSummary
/logs                    -> LogViewer
/settings                -> Settings form (device defaults, ReplayGain mode, scan paths)
```

```tsx
// src/routes/albums/$albumId.tsx
import { createFileRoute } from "@tanstack/react-router";
import { albumQueryOptions } from "@/hooks/useAlbums";

export const Route = createFileRoute("/albums/$albumId")({
  loader: ({ context: { queryClient }, params }) =>
    queryClient.ensureQueryData(albumQueryOptions(Number(params.albumId))),
  component: AlbumDetailView,
});

function AlbumDetailView() {
  const { albumId } = Route.useParams();
  const { data: album } = useSuspenseQuery(albumQueryOptions(Number(albumId)));
  return <TrackTable tracks={album.tracks} />;
}
```

Route loaders call `queryClient.ensureQueryData` so navigation and data fetching are coupled at the router level — no `useEffect`-driven fetches inside route components, and no loading spinner flash for data already in the cache (revisiting an album is instant).

## 3. State split rule

This is the single most important architectural rule in the frontend and it is enforced without exception:

> **TanStack Query owns all server/library data. Zustand owns only ephemeral UI and realtime state. Library data is never duplicated into Zustand.**

**TanStack Query** — albums, artists, tracks, playlists, stats, settings. Anything that originates from `signal-db` and is fetched via an IPC command. Keyed consistently by domain and id:

```typescript
// src/hooks/useAlbums.ts
export const albumQueryOptions = (albumId: number) =>
  queryOptions({
    queryKey: ["album", albumId] as const,
    queryFn: () => libraryGetAlbum(albumId),
  });

export const albumsListQueryOptions = (params: AlbumListParams) =>
  queryOptions({
    queryKey: ["albums", params] as const,
    queryFn: () => libraryListAlbums(params),
  });

export const playlistQueryOptions = (playlistId: number) =>
  queryOptions({
    queryKey: ["playlist", playlistId] as const,
    queryFn: () => playlistGet(playlistId),
  });

export const statsQueryOptions = (range: StatsRange) =>
  queryOptions({
    queryKey: ["stats", range] as const,
    queryFn: () => statsOverview(range),
  });
```

These keys are invalidated by backend events, not by manual refetch calls scattered through components:

```typescript
// src/lib/ipc/events.ts (excerpt — full listing in §4)
listen<ScannerDone>("scanner:done", () => {
  queryClient.invalidateQueries({ queryKey: ["albums"] });
  queryClient.invalidateQueries({ queryKey: ["artists"] });
  queryClient.invalidateQueries({ queryKey: ["album"] }); // any open album detail
});

listen<QueueChanged>("queue:changed", (event) => {
  queueStore.getState().setItems(event.payload.items);
  // queue itself is Zustand (realtime), but playlists derived FROM a save
  // action invalidate the playlists query separately, on command resolution
});
```

**Zustand** — three stores, each holding only state that either (a) has no server-side identity at all (focused pane, palette open/closed, current multi-select), or (b) is a live mirror of backend push events that would be wasteful or laggy to route through Query's cache-and-refetch model.

```typescript
// src/stores/playerStore.ts
interface PlayerStore {
  state: "stopped" | "playing" | "paused";
  currentTrackId: number | null;
  positionMs: number;
  durationMs: number;
  device: AudioDevice | null;
  setFromPlayerState: (payload: PlayerStateChanged) => void;
  setFromProgress: (payload: PlayerProgress) => void;
  setFromTrackChanged: (payload: TrackChanged) => void;
  setFromDeviceChanged: (payload: DeviceChanged) => void;
}

export const usePlayerStore = create<PlayerStore>((set) => ({
  state: "stopped",
  currentTrackId: null,
  positionMs: 0,
  durationMs: 0,
  device: null,
  setFromPlayerState: (p) => set({ state: p.state, currentTrackId: p.trackId }),
  setFromProgress: (p) => set({ positionMs: p.positionMs, durationMs: p.durationMs }),
  setFromTrackChanged: (p) => set({ currentTrackId: p.track.id, durationMs: p.track.durationMs }),
  setFromDeviceChanged: (p) => set({ device: p.device }),
}));
```

```typescript
// src/stores/uiStore.ts
interface UiStore {
  focusedPane: "left" | "center" | "right" | "bottom";
  paletteOpen: boolean;
  selection: Set<number>; // selected track ids in the current CENTER view
  setFocusedPane: (pane: UiStore["focusedPane"]) => void;
  togglePalette: () => void;
  setSelection: (ids: Set<number>) => void;
}
```

```typescript
// src/stores/queueStore.ts
interface QueueStore {
  items: QueueItem[];
  setItems: (items: QueueItem[]) => void; // always a full replace, never a splice
}
```

Note that `queueStore` holds `QueueItem[]` (ephemeral/orderable session state pushed by `queue:changed`), not `Track[]` or `Playlist[]` — the moment something has a durable server identity fetched by id, it belongs in Query. `QueuePanel` reads queue order from `queueStore` and joins against `useQuery(['track', id])` per row, never by copying track fields into the queue store itself.

## 4. Event bridge

A single module, `src/lib/ipc/events.ts`, subscribes to every backend event exactly once at application startup. No component or hook calls `listen()` directly — this keeps subscription lifecycle out of the render tree entirely and makes "what listens to what" auditable in one file.

```typescript
// src/lib/ipc/events.ts
import { listen } from "@tauri-apps/api/event";
import type { QueryClient } from "@tanstack/react-query";
import { usePlayerStore } from "@/stores/playerStore";
import { useQueueStore } from "@/stores/queueStore";
import { useLogStore } from "@/stores/logStore";
import type {
  PlayerStateChanged,
  PlayerProgress,
  TrackChanged,
  DeviceChanged,
  ScannerProgress,
  ScannerDone,
  QueueChanged,
  LogLine,
} from "./types";

export async function bootstrapEventBridge(queryClient: QueryClient) {
  const unlisteners = await Promise.all([
    listen<PlayerStateChanged>("player:state", (e) =>
      usePlayerStore.getState().setFromPlayerState(e.payload),
    ),
    listen<PlayerProgress>("player:progress", (e) =>
      usePlayerStore.getState().setFromProgress(e.payload),
    ),
    listen<TrackChanged>("player:track-changed", (e) =>
      usePlayerStore.getState().setFromTrackChanged(e.payload),
    ),
    listen<DeviceChanged>("player:device-changed", (e) =>
      usePlayerStore.getState().setFromDeviceChanged(e.payload),
    ),
    listen<ScannerProgress>("scanner:progress", (e) =>
      useUiStore.getState().setScanProgress(e.payload),
    ),
    listen<ScannerDone>("scanner:done", () => {
      queryClient.invalidateQueries({ queryKey: ["albums"] });
      queryClient.invalidateQueries({ queryKey: ["artists"] });
      queryClient.invalidateQueries({ queryKey: ["album"] });
      useUiStore.getState().clearScanProgress();
    }),
    listen<QueueChanged>("queue:changed", (e) =>
      useQueueStore.getState().setItems(e.payload.items),
    ),
    listen<LogLine>("log:line", (e) => useLogStore.getState().push(e.payload)),
  ]);

  return () => unlisteners.forEach((unlisten) => unlisten());
}
```

```tsx
// src/main.tsx
const queryClient = new QueryClient();

bootstrapEventBridge(queryClient); // fire-and-forget at startup, before first render matters

const router = createRouter({ routeTree, context: { queryClient } });

ReactDOM.createRoot(document.getElementById("root")!).render(
  <QueryClientProvider client={queryClient}>
    <RouterProvider router={router} />
  </QueryClientProvider>,
);
```

`bootstrapEventBridge` is called once, outside any component, so there is exactly one WebView-side listener per event name for the lifetime of the app — no duplicate subscriptions from remounts, no cleanup races.

## 5. Component breakdown

| Component | Responsibility |
|---|---|
| `AppShell` | Renders the fixed three-column + bottom-bar grid; accepts `left`/`center`/`right`/`bottom` slots and lays them out with CSS grid; owns no state itself. |
| `LibraryNav` | LEFT pane: Artists/Albums/Genres/Folders navigation tree; drives router navigation on selection, highlights the active route. |
| `MainView` | Thin wrapper around the router's `<Outlet/>` for the CENTER pane; supplies scroll-region and empty-state handling shared across all routes. |
| `InspectorPane` | RIGHT pane, top half: renders `TrackTechnical` (codec, bit depth, sample rate, ReplayGain, peak, DR) for the currently playing or currently selected track, plus current output device and bit-perfect status. |
| `QueuePanel` | RIGHT pane, bottom half: renders the live queue from `queueStore`, supports reorder (drag or keyboard), remove, and "save as playlist". |
| `StatusBar` | BOTTOM pane, thin strip: shows transient status text (scan progress, last command result, connection state). |
| `CommandLine` | BOTTOM pane: single-line input that is the entry point for both instant navigation shortcuts and full palette commands (see §7). |
| `PaletteOverlay` | Full command palette (shadcn `Command` component) triggered by a keybinding; shares the execution path and fuzzy matcher with `CommandLine`. |
| `TrackTable` | Virtualized, dense monospace row grid for track listings, used by album detail, playlist, and search routes; see §6. |
| `LogViewer` | Renders `logs_tail` backfill plus live `log:line` events from `logStore`, with level-based filtering/highlighting. |
| `StatsHeatmap` | uPlot-based calendar heatmap of listening activity, driven by `stats_overview`. |

## 6. `TrackTable`: virtualization and selection model

`TrackTable` is the one component in the app that has to comfortably render libraries with 100k+ tracks, so it never renders more than a viewport's worth of DOM rows.

```tsx
// src/components/library/TrackTable.tsx
import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";
import { useUiStore } from "@/stores/uiStore";

interface TrackTableProps {
  tracks: Track[];
}

export function TrackTable({ tracks }: TrackTableProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const selection = useUiStore((s) => s.selection);
  const cursor = useUiStore((s) => s.cursorIndex);

  const rowVirtualizer = useVirtualizer({
    count: tracks.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 28, // px, dense row height
    overscan: 16,
  });

  return (
    <div ref={scrollRef} className="h-full overflow-y-auto font-mono text-sm">
      <div style={{ height: rowVirtualizer.getTotalSize(), position: "relative" }}>
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const track = tracks[virtualRow.index];
          return (
            <TrackRow
              key={track.id}
              track={track}
              selected={selection.has(track.id)}
              isCursor={virtualRow.index === cursor}
              style={{
                position: "absolute",
                top: 0,
                transform: `translateY(${virtualRow.start}px)`,
                height: virtualRow.size,
              }}
            />
          );
        })}
      </div>
    </div>
  );
}
```

Each row is a dense grid (`grid-cols-[auto_1fr_auto_auto_auto]` or similar) with monospace-aligned columns for track number, title/artist, duration, codec badge, and bitrate — no per-cell React state, purely props-driven from the `Track` DTO.

Selection and cursor position are **not** DOM focus. Vim-style `j`/`k` navigation moves a `cursorIndex` integer in `uiStore`; `TrackRow` reads whether it's the cursor row or part of `selection` purely from store state and applies highlight classes. This avoids per-row `element.focus()` calls and avoids the DOM losing track of focus across virtualization remounts, since recycled/unmounted rows don't preserve native focus. The keyboard layer (§7) mutates `cursorIndex`; `rowVirtualizer.scrollToIndex(cursor)` runs imperatively in a `useEffect` keyed on `cursor` to keep it in view.

## 7. Keyboard layer

The keyboard layer is a single global `keydown` handler mounted once in `AppShell`, structured as a **mode stack** plus a **pane-scoped binding registry**. The full binding list (which key does what in which mode/pane) is specified in `docs/09-keyboard.md` — this section only covers the architecture that executes those bindings, not the bindings themselves.

```typescript
// src/lib/keyboard/keymap.ts
type Mode = "normal" | "palette" | "search" | "rename";

interface KeymapState {
  modeStack: Mode[]; // top of stack = active mode; push on palette/search open, pop on close/escape
}

type Pane = "left" | "center" | "right" | "bottom";

interface Binding {
  key: string;             // e.g. "j", "g g", "ctrl+p"
  mode: Mode;
  pane?: Pane;              // undefined = global within that mode
  handler: () => void;
}

class KeymapRegistry {
  private bindings: Binding[] = [];

  register(binding: Binding) {
    this.bindings.push(binding);
  }

  resolve(key: string, mode: Mode, focusedPane: Pane): Binding | undefined {
    return (
      this.bindings.find((b) => b.key === key && b.mode === mode && b.pane === focusedPane) ??
      this.bindings.find((b) => b.key === key && b.mode === mode && b.pane === undefined)
    );
  }
}

export const keymapRegistry = new KeymapRegistry();
```

The active mode is always `modeStack[modeStack.length - 1]`. Opening the palette pushes `"palette"`; `Escape` or successful execution pops it, returning focus to whatever mode was underneath (always `"normal"` in practice, but the stack shape leaves room for nested modes like `"rename"`). Pane-scoped bindings resolve against `uiStore.focusedPane` first and fall back to global bindings for that mode, so `j`/`k` can mean "move cursor in `TrackTable`" when CENTER is focused and "move selection in `QueuePanel`" when RIGHT is focused, without either binding knowing about the other.

Components register their bindings declaratively via a hook, scoped to their own lifetime:

```typescript
// src/hooks/useKeymap.ts
export function usePaneBindings(pane: Pane, mode: Mode, bindings: Omit<Binding, "mode" | "pane">[]) {
  useEffect(() => {
    const registered = bindings.map((b) => ({ ...b, mode, pane }));
    registered.forEach((b) => keymapRegistry.register(b));
    return () => registered.forEach((b) => keymapRegistry.unregister(b));
  }, [pane, mode, bindings]);
}
```

## 8. Command palette and command line

`CommandLine` (BOTTOM, always visible, single line) and `PaletteOverlay` (full modal, opened on keybinding) share one execution path — the only difference is presentation. Both call the same `executeCommand(raw: string)` function.

```typescript
// src/lib/search/commandRegistry.ts
interface CommandDef {
  name: string;               // e.g. "albums", "artists", "search", "stats", "logs", "settings"
  kind: "navigate";           // client-resolvable commands are all instant navigation
  route: (args: string[]) => string;
}

const CLIENT_COMMANDS: CommandDef[] = [
  { name: "albums", kind: "navigate", route: () => "/albums" },
  { name: "artists", kind: "navigate", route: () => "/artists" },
  { name: "search", kind: "navigate", route: (args) => `/search?q=${encodeURIComponent(args.join(" "))}` },
  { name: "stats", kind: "navigate", route: () => "/stats" },
  { name: "logs", kind: "navigate", route: () => "/logs" },
  { name: "settings", kind: "navigate", route: () => "/settings" },
];

export function matchClientCommand(raw: string): CommandDef | undefined {
  const [name] = raw.trim().split(/\s+/);
  return CLIENT_COMMANDS.find((c) => c.name === name);
}
```

```typescript
// src/lib/search/executeCommand.ts
export async function executeCommand(raw: string, router: Router): Promise<void> {
  const client = matchClientCommand(raw);
  if (client) {
    const [, ...args] = raw.trim().split(/\s+/);
    router.navigate({ to: client.route(args) });
    return;
  }

  try {
    const result = await paletteExecute(raw);
    switch (result.kind) {
      case "navigate":
        router.navigate({ to: result.payload.route });
        break;
      case "feedback":
        toast({ title: result.payload.message });
        break;
      case "error":
        useUiStore.getState().setPaletteError(result.payload);
        break;
    }
  } catch (err) {
    reportIpcError(err);
  }
}
```

Pure navigation shortcuts (`:albums`, `:artists`, `:search <query>`, `:stats`, `:logs`, `:settings`) never touch IPC — they resolve client-side for zero-latency navigation, since the frontend already knows every route in the app. Anything with backend semantics (`play bocanada`, `device topping`, `scan ~/Music`, arbitrary search-query-language input) falls through to `palette_execute` (see `docs/05-ipc-api.md` §7), since only the backend knows about track/device/library state.

Fuzzy matching over the command registry (for the palette's autocomplete list, not for dispatch) runs entirely client-side against `CLIENT_COMMANDS` plus a static list of backend command names/descriptions bundled at build time, using a small substring/subsequence scorer — no IPC round-trip is needed just to populate the suggestion list as the user types.

## 9. Performance rules

- **No data fetching below route level.** Only route `loader`s and the top-level route component call `useQuery`/`useSuspenseQuery`. Child components receive data as props. This keeps the query-cache-to-component data flow traceable to one place per route and prevents waterfalls from nested components each issuing their own fetch.
- **Memoized selectors on Zustand.** Components subscribe to stores with narrow, memoized selectors (`usePlayerStore((s) => s.state)`, not `usePlayerStore()`), so a `positionMs` update at 4 Hz does not re-render components that only care about `state` or `device`. Selectors that derive computed values (e.g. formatted time strings) are wrapped in `useShallow` or a `createSelector`-style memoizer to avoid a fresh object identity on every store update.
- **uPlot for charts.** `StatsHeatmap` and any future charting use uPlot directly (imperative canvas rendering, one mount, manual updates), not a React-idiomatic charting library. React-wrapped chart libraries that re-render an SVG tree on every data change are unacceptable for anything fed by realtime events; uPlot's imperative `setData` call sidesteps React's render cycle entirely for the chart body.
- **Progress bypasses React state where possible.** The seekbar and elapsed/remaining time readout do not re-render through React on every `player:progress` tick. Instead, the event bridge writes position updates directly to a DOM ref (`seekbarRef.current.style.setProperty('--progress', ...)`, `timeRef.current.textContent = formatTime(positionMs)`) inside the `listen()` callback, bypassing `setState` entirely for this one high-frequency path. `playerStore.positionMs` is still updated (for components that do need reactive access, like the palette showing "now playing"), but the playback bar itself reads the ref-driven DOM update path so a 4 Hz tick never triggers a React commit on the hot path.

## 10. Styling

Signal uses Tailwind CSS with shadcn/ui components as the primitive layer, and a single fixed dark theme — there is no light mode and no theme switcher. All color, spacing, and typography tokens (the terminal-inspired monospace palette, focus-ring treatment, pane border colors) are defined once as CSS variables and documented in `docs/10-design-system.md`; this document does not duplicate those values. Components consume tokens exclusively through Tailwind utility classes mapped to those CSS variables (`bg-background`, `text-foreground`, `border-border`, etc.) — no component hardcodes a hex color or inline style for anything covered by the design system.
