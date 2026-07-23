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

export interface DayCount {
  day: string;
  plays: number;
}

export interface NameCount {
  name: string;
  count: number;
}

export interface StatsOverview {
  totalPlays: number;
  totalMsPlayed: number;
  distinctTracks: number;
  heatmap: DayCount[];
  topArtists: NameCount[];
  topCodecs: NameCount[];
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
