import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { toast } from "@/stores/toastStore";

/** Show a track's file in the OS file manager — the 'o' binding and the row
 *  context menu both land here. A missing file is the common failure: the
 *  library row outlives the file it was scanned from. */
export function revealTrack(track: Track | undefined) {
  if (!track) return;
  void api
    .revealFile(track.technical.filePath)
    .catch(() => toast.error("file not found"));
}
