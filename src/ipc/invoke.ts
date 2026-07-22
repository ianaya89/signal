import { invoke } from "@tauri-apps/api/core";

import type { AlbumDetail, AlbumSummary, ArtistSummary } from "@/ipc/types";

// Grows with each milestone; keep in sync with src-tauri commands.
export type IpcCommand =
  | "settings_get"
  | "settings_set"
  | "library_scan"
  | "library_list_albums"
  | "library_list_artists"
  | "library_get_album";

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
  settingsGet: (key: string) => ipc<string | null>("settings_get", { key }),
  settingsSet: (key: string, value: string) =>
    ipc<void>("settings_set", { key, value }),
};
