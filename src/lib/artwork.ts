const isWindows = navigator.userAgent.includes("Windows");

// Custom-protocol URL shape differs per platform (docs/05-ipc-api.md).
export function artworkUrl(albumId: number): string {
  return isWindows
    ? `http://signal-art.localhost/album/${albumId}`
    : `signal-art://localhost/album/${albumId}`;
}
