import type { QueryClient } from "@tanstack/react-query";

import { onSignalEvent } from "@/ipc/events";
import { startupUpdateCheck } from "@/lib/updater";
import type {
  AnalysisDoneEvent,
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
import { toast } from "@/stores/toastStore";
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

  startupUpdateCheck();

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

  // resume where the last session left off (paused)
  void api
    .sessionRestore()
    .then((resume) => {
      if (resume) {
        toast.info("resumed — press space to continue");
      }
    })
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

  // refresh stored verdicts even when the doctor view isn't mounted
  void onSignalEvent<AnalysisDoneEvent>("analysis:done", () => {
    void queryClient.invalidateQueries({ queryKey: ["analysis"] });
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

  // config.toml [ui] section: theme + custom accent, applied live
  void onSignalEvent<{ ui: string }>("config:changed", (e) => {
    try {
      const ui: unknown = JSON.parse(e.ui);
      if (typeof ui !== "object" || ui === null) return;
      const { theme, accent } = ui as { theme?: string; accent?: string };
      if (theme === "dark" || theme === "light") {
        useUiStore.getState().setTheme(theme, false);
      }
      const root = document.documentElement;
      if (accent && /^#[0-9a-fA-F]{6}$/.test(accent)) {
        root.style.setProperty("--accent", accent);
        root.style.setProperty("--border-focus", accent);
      } else {
        root.style.removeProperty("--accent");
        root.style.removeProperty("--border-focus");
      }
    } catch {
      // malformed config event: ignore
    }
  });
}
