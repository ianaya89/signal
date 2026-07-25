import { invoke } from "@tauri-apps/api/core";

import type {
  AlbumDetail,
  AlbumSummary,
  AppInfo,
  ArtistDetail,
  ArtistSummary,
  AudioDevice,
  FolderListing,
  GenreSummary,
  HealthReport,
  PlayMode,
  PlaylistDetail,
  PlaylistSummary,
  QueueEntry,
  ReplayGainMode,
  StatsOverview,
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
  | "library_get_artist"
  | "library_reset_and_rescan"
  | "player_play"
  | "player_play_context"
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
  | "search_query"
  | "player_next"
  | "player_prev"
  | "stats_overview"
  | "device_list"
  | "device_select"
  | "player_set_replaygain"
  | "player_set_exclusive"
  | "playlist_list"
  | "playlist_get"
  | "playlist_create"
  | "playlist_rename"
  | "playlist_delete"
  | "playlist_add_tracks"
  | "playlist_remove_track"
  | "queue_save_as_playlist"
  | "library_rename_artist"
  | "library_rename_album"
  | "library_rename_track"
  | "library_set_album_artwork"
  | "track_set_rating"
  | "track_toggle_favorite"
  | "player_set_mode"
  | "player_get_mode"
  | "queue_add_next"
  | "library_list_genres"
  | "library_get_genre_tracks"
  | "reveal_in_file_manager"
  | "library_browse_folder"
  | "session_restore"
  | "app_info"
  | "smart_playlist_create"
  | "smart_playlist_update"
  | "smart_playlist_delete"
  | "smart_playlist_rules"
  | "plugin_set_listenbrainz"
  | "plugin_status"
  | "library_health"
  | "library_prune_missing"
  | "playlist_export_m3u"
  | "library_backup";

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
  getArtist: (artistId: number) =>
    ipc<ArtistDetail>("library_get_artist", { artistId }),
  resetAndRescan: () => ipc<void>("library_reset_and_rescan"),
  settingsGet: (key: string) => ipc<string | null>("settings_get", { key }),
  settingsSet: (key: string, value: string) =>
    ipc<void>("settings_set", { key, value }),
  play: (trackId: number) => ipc<void>("player_play", { trackId }),
  playContext: (trackIds: number[], startIndex: number) =>
    ipc<void>("player_play_context", { trackIds, startIndex }),
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
  next: () => ipc<boolean>("player_next"),
  prev: () => ipc<void>("player_prev"),
  statsOverview: () => ipc<StatsOverview>("stats_overview"),
  deviceList: () => ipc<AudioDevice[]>("device_list"),
  deviceSelect: (deviceId: string) => ipc<void>("device_select", { deviceId }),
  setReplaygain: (mode: ReplayGainMode) =>
    ipc<void>("player_set_replaygain", { mode }),
  setExclusive: (exclusive: boolean) =>
    ipc<void>("player_set_exclusive", { exclusive }),
  playlistList: () => ipc<PlaylistSummary[]>("playlist_list"),
  playlistGet: (playlistId: number, smart: boolean) =>
    ipc<PlaylistDetail>("playlist_get", { playlistId, smart }),
  playlistCreate: (name: string) => ipc<number>("playlist_create", { name }),
  playlistRename: (playlistId: number, name: string) =>
    ipc<void>("playlist_rename", { playlistId, name }),
  playlistDelete: (playlistId: number) =>
    ipc<void>("playlist_delete", { playlistId }),
  playlistAddTracks: (playlistId: number, trackIds: number[]) =>
    ipc<void>("playlist_add_tracks", { playlistId, trackIds }),
  playlistRemoveTrack: (playlistId: number, trackId: number) =>
    ipc<void>("playlist_remove_track", { playlistId, trackId }),
  queueSaveAsPlaylist: (name: string) =>
    ipc<number>("queue_save_as_playlist", { name }),
  renameArtist: (artistId: number, name: string) =>
    ipc<boolean>("library_rename_artist", { artistId, name }),
  renameAlbum: (albumId: number, name: string) =>
    ipc<boolean>("library_rename_album", { albumId, name }),
  renameTrack: (trackId: number, title: string) =>
    ipc<void>("library_rename_track", { trackId, title }),
  setAlbumArtwork: (albumId: number, sourcePath: string) =>
    ipc<void>("library_set_album_artwork", { albumId, sourcePath }),
  setRating: (trackId: number, rating: number) =>
    ipc<void>("track_set_rating", { trackId, rating }),
  toggleFavorite: (trackId: number) =>
    ipc<boolean>("track_toggle_favorite", { trackId }),
  setPlayMode: (mode: PlayMode) => ipc<void>("player_set_mode", { mode }),
  getPlayMode: () => ipc<PlayMode>("player_get_mode"),
  queueAddNext: (trackId: number) => ipc<void>("queue_add_next", { trackId }),
  listGenres: () => ipc<GenreSummary[]>("library_list_genres"),
  genreTracks: (genreId: number) =>
    ipc<Track[]>("library_get_genre_tracks", { genreId }),
  revealFile: (path: string) => ipc<void>("reveal_in_file_manager", { path }),
  browseFolder: (path?: string) =>
    ipc<FolderListing>("library_browse_folder", { path }),
  sessionRestore: () =>
    ipc<{ trackId: number; positionMs: number } | null>("session_restore"),
  appInfo: () => ipc<AppInfo>("app_info"),
  smartCreate: (name: string, rules: string) =>
    ipc<number>("smart_playlist_create", { name, rules }),
  smartUpdate: (playlistId: number, name: string, rules: string) =>
    ipc<void>("smart_playlist_update", { playlistId, name, rules }),
  smartDelete: (playlistId: number) =>
    ipc<void>("smart_playlist_delete", { playlistId }),
  smartRules: (playlistId: number) =>
    ipc<string | null>("smart_playlist_rules", { playlistId }),
  setListenBrainz: (token: string) =>
    ipc<boolean>("plugin_set_listenbrainz", { token }),
  pluginStatus: () => ipc<{ listenbrainz: boolean }>("plugin_status"),
  libraryHealth: () => ipc<HealthReport>("library_health"),
  libraryPruneMissing: (trackIds: number[]) =>
    ipc<number>("library_prune_missing", { trackIds }),
  exportM3u: (playlistId: number, smart: boolean, destPath: string) =>
    ipc<number>("playlist_export_m3u", { playlistId, smart, destPath }),
  libraryBackup: (destPath: string) =>
    ipc<void>("library_backup", { destPath }),
};
