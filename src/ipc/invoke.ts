import { invoke } from "@tauri-apps/api/core";

import type {
  AlbumDetail,
  AlbumSummary,
  ArtistSummary,
  QueueEntry,
  Track,
  TrackWithContext,
} from "@/ipc/types";

// Grows with each milestone; keep in sync with src-tauri commands.
export type IpcCommand =
  | "settings_get"
  | "settings_set"
  | "library_scan"
  | "library_list_albums"
  | "library_list_artists"
  | "library_get_album"
  | "library_get_track"
  | "player_play"
  | "player_toggle"
  | "player_pause"
  | "player_stop"
  | "player_seek"
  | "player_set_volume"
  | "player_get_state"
  | "queue_list"
  | "queue_add"
  | "queue_remove"
  | "queue_move"
  | "queue_clear"
  | "queue_play_next"
  | "search_query";

export function ipc<T>(
  command: IpcCommand,
  args?: Record<string, unknown>,
): Promise<T> {
  return invoke<T>(command, args);
}

export const api = {
  scanLibrary: (root: string) => ipc<void>("library_scan", { root }),
  listAlbums: () => ipc<AlbumSummary[]>("library_list_albums"),
  listArtists: () => ipc<ArtistSummary[]>("library_list_artists"),
  getAlbum: (albumId: number) => ipc<AlbumDetail>("library_get_album", { albumId }),
  getTrack: (trackId: number) =>
    ipc<TrackWithContext>("library_get_track", { trackId }),
  settingsGet: (key: string) => ipc<string | null>("settings_get", { key }),
  settingsSet: (key: string, value: string) =>
    ipc<void>("settings_set", { key, value }),
  play: (trackId: number) => ipc<void>("player_play", { trackId }),
  toggle: () => ipc<void>("player_toggle"),
  stop: () => ipc<void>("player_stop"),
  seek: (positionMs: number) => ipc<void>("player_seek", { positionMs }),
  setVolume: (volume: number) => ipc<void>("player_set_volume", { volume }),
  queueList: () => ipc<QueueEntry[]>("queue_list"),
  queueAdd: (trackId: number) => ipc<void>("queue_add", { trackId }),
  queueRemove: (queueItemId: number) =>
    ipc<void>("queue_remove", { queueItemId }),
  queueMove: (orderedIds: number[]) => ipc<void>("queue_move", { orderedIds }),
  queueClear: () => ipc<void>("queue_clear"),
  queuePlayNext: () => ipc<boolean>("queue_play_next"),
  search: (query: string) => ipc<Track[]>("search_query", { query }),
};
