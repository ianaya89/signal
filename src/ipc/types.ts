// DTOs mirroring signal-core models (serde camelCase). Keep in sync manually
// until ts-rs/specta generation lands (docs/05-ipc-api.md).

export interface TrackTechnical {
  codec: string;
  container: string;
  bitrateKbps: number;
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

export interface Track {
  id: number;
  title: string;
  artistId: number;
  albumId: number;
  trackNo: number | null;
  discNo: number | null;
  year: number | null;
  durationMs: number;
  rating: number | null;
  favorite: boolean;
  playCount: number;
  skipCount: number;
  addedAt: string;
  modifiedAt: string;
  lastPlayedAt: string | null;
  technical: TrackTechnical;
}

export interface AlbumSummary {
  id: number;
  name: string;
  artistId: number;
  artistName: string;
  year: number | null;
  artworkPath: string | null;
  trackCount: number;
}

export interface ArtistSummary {
  id: number;
  name: string;
  albumCount: number;
  trackCount: number;
}

export interface AlbumDetail {
  album: AlbumSummary;
  tracks: Track[];
}

export interface ArtistDetail {
  artist: ArtistSummary;
  albums: AlbumSummary[];
}

export interface TrackWithContext {
  track: Track;
  artistName: string;
  albumName: string;
  genre: string | null;
}

export interface TrackMetaEdit {
  title: string;
  artist: string;
  album: string;
  year: number | null;
  trackNo: number | null;
  discNo: number | null;
  genre: string | null;
}

export interface QueueItem {
  id: number;
  position: number;
  trackId: number;
  addedAt: string;
}

export interface QueueEntry {
  item: QueueItem;
  track: Track;
}

export interface PlaylistSummary {
  id: number;
  name: string;
  trackCount: number;
  smart: boolean;
}

export interface PlaylistDetail {
  id: number;
  name: string;
  smart: boolean;
  tracks: Track[];
}

export interface AudioDevice {
  id: string;
  name: string;
  backend: string;
}

export type ReplayGainMode = "off" | "track" | "album";

export type RepeatMode = "off" | "all" | "one";

export interface PlayMode {
  shuffle: boolean;
  repeat: RepeatMode;
}

export interface DayCount {
  day: string;
  plays: number;
}

export interface NameCount {
  name: string;
  count: number;
}

export interface AlbumPlayCount {
  albumId: number;
  name: string;
  artistName: string;
  plays: number;
}

export interface StatsOverview {
  totalPlays: number;
  totalMsPlayed: number;
  distinctTracks: number;
  heatmap: DayCount[];
  topArtists: NameCount[];
  topCodecs: NameCount[];
  topAlbums: AlbumPlayCount[];
  hourly: number[];
}

export interface GenreSummary {
  id: number;
  name: string;
  trackCount: number;
}

export interface AppInfo {
  version: string;
  dbPath: string;
  cacheDir: string;
  libraryRoot: string | null;
  trackCount: number;
}

export interface SmartCondition {
  field: string;
  op: string;
  value: string | number | boolean;
}

export interface SmartRules {
  match: "all" | "any";
  conditions: SmartCondition[];
  order_by?: string;
  order_dir?: "asc" | "desc";
  limit?: number | null;
}

export interface HealthTrackRef {
  id: number;
  title: string;
  artistName: string;
  albumId: number;
  detail: string;
}

export interface HealthReport {
  totalTracks: number;
  losslessPct: number;
  score: number;
  missingFiles: HealthTrackRef[];
  missingFilesTotal: number;
  duplicates: {
    title: string;
    artistName: string;
    count: number;
    trackIds: number[];
  }[];
  duplicatesTotal: number;
  albumsWithoutArt: { id: number; name: string; artistName: string }[];
  albumsWithoutArtTotal: number;
  tracksWithoutYear: number;
  tracksWithoutGenre: number;
  lowBitrate: HealthTrackRef[];
  lowBitrateTotal: number;
}

export interface FolderEntry {
  name: string;
  path: string;
  trackCount: number;
}

export interface FolderListing {
  root: string;
  path: string;
  dirs: FolderEntry[];
  tracks: Track[];
}

export interface ScannerProgressEvent {
  processed: number;
  total: number;
  currentPath: string;
}

export interface ScannerDoneEvent {
  added: number;
  updated: number;
  removed: number;
  skipped: number;
  errors: number;
}

export interface ScannerErrorEvent {
  message: string;
}
