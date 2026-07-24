import type { QueryClient } from "@tanstack/react-query";

import { onSignalEvent } from "@/ipc/events";
import type {
  ScannerDoneEvent,
  ScannerErrorEvent,
  ScannerProgressEvent,
} from "@/ipc/types";
import { api } from "@/ipc/invoke";
import { useLogStore } from "@/stores/logStore";
import { usePlayModeStore } from "@/stores/playModeStore";
import type { PlayerStateDto } from "@/stores/playerStore";
import { usePlayerStore } from "@/stores/playerStore";
import { useQueueStore } from "@/stores/queueStore";
import { useScanStore } from "@/stores/scanStore";
import { useUiStore } from "@/stores/uiStore";

interface ProgressEvent {
  positionMs: number;
  durationMs: number;
}

/// Single subscription point for backend events; called once at startup.
export function bootstrapEvents(queryClient: QueryClient) {
  if (!("__TAURI_INTERNALS__" in window)) {
    return; // plain-browser dev: no tauri runtime, no events
  }

  // restore persisted theme without re-persisting it
  void api
    .settingsGet("ui.theme")
    .then((theme) => {
      if (theme === "light" || theme === "dark") {
        useUiStore.getState().setTheme(theme, false);
      }
    })
    .catch(() => {});

  void api
    .getPlayMode()
    .then((mode) => usePlayModeStore.getState().restore(mode))
    .catch(() => {});

  void api
    .settingsGet("ui.layout")
    .then((raw) => {
      if (!raw) return;
      const layout: unknown = JSON.parse(raw);
      if (typeof layout === "object" && layout !== null) {
        useUiStore.getState().restoreLayout(layout);
      }
    })
    .catch(() => {});

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

  void onSignalEvent<{ level: string; target: string; message: string }>(
    "log:line",
    (line) => {
      useLogStore.getState().push(line);
    },
  );
}
