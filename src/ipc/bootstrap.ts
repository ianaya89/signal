import type { QueryClient } from "@tanstack/react-query";

import { onSignalEvent } from "@/ipc/events";
import type {
  ScannerDoneEvent,
  ScannerErrorEvent,
  ScannerProgressEvent,
} from "@/ipc/types";
import type { PlayerStateDto } from "@/stores/playerStore";
import { usePlayerStore } from "@/stores/playerStore";
import { useQueueStore } from "@/stores/queueStore";
import { useScanStore } from "@/stores/scanStore";

interface ProgressEvent {
  positionMs: number;
  durationMs: number;
}

/// Single subscription point for backend events; called once at startup.
export function bootstrapEvents(queryClient: QueryClient) {
  if (!("__TAURI_INTERNALS__" in window)) {
    return; // plain-browser dev: no tauri runtime, no events
  }

  void onSignalEvent<ScannerProgressEvent>("scanner:progress", (p) => {
    useScanStore.getState().progress(p);
  });

  void onSignalEvent<ScannerDoneEvent>("scanner:done", (d) => {
    useScanStore.getState().done(d);
    void queryClient.invalidateQueries();
  });

  void onSignalEvent<ScannerErrorEvent>("scanner:error", (e) => {
    useScanStore.getState().fail(e.message);
  });

  void onSignalEvent<{ state: PlayerStateDto }>("player:state", (e) => {
    usePlayerStore.getState().applyState(e.state);
  });

  void onSignalEvent<ProgressEvent>("player:progress", (e) => {
    usePlayerStore.getState().applyProgress(e.positionMs, e.durationMs);
  });

  void onSignalEvent("queue:changed", () => {
    void useQueueStore.getState().refresh();
  });
  void useQueueStore.getState().refresh();
}
