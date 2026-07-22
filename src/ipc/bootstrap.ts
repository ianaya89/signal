import type { QueryClient } from "@tanstack/react-query";

import { onSignalEvent } from "@/ipc/events";
import type { ScannerDoneEvent, ScannerProgressEvent } from "@/ipc/types";
import { useScanStore } from "@/stores/scanStore";

/// Single subscription point for backend events; called once at startup.
export function bootstrapEvents(queryClient: QueryClient) {
  if (!("__TAURI_INTERNALS__" in window)) {
    return; // plain-browser dev: no tauri runtime, no events
  }

  void onSignalEvent<ScannerProgressEvent>("scanner:progress", (p) => {
    useScanStore.getState().progress(p);
  });

  void onSignalEvent<ScannerDoneEvent>("scanner:done", () => {
    useScanStore.getState().done();
    void queryClient.invalidateQueries();
  });
}
