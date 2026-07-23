import { create } from "zustand";

import type { ScannerDoneEvent, ScannerProgressEvent } from "@/ipc/types";

interface ScanState {
  scanning: boolean;
  processed: number;
  total: number;
  currentPath: string;
  lastError: string | null;
  /// Post-scan summary line shown until the next action.
  summary: string | null;
  start: () => void;
  progress: (p: ScannerProgressEvent) => void;
  done: (d: ScannerDoneEvent) => void;
  fail: (message: string) => void;
}

export const useScanStore = create<ScanState>((set) => ({
  scanning: false,
  processed: 0,
  total: 0,
  currentPath: "",
  lastError: null,
  summary: null,
  start: () =>
    set({
      scanning: true,
      processed: 0,
      total: 0,
      currentPath: "",
      lastError: null,
      summary: null,
    }),
  progress: (p) =>
    set({
      scanning: true,
      processed: p.processed,
      total: p.total,
      currentPath: p.currentPath,
    }),
  done: (d) =>
    set({
      scanning: false,
      currentPath: "",
      summary: `scan done: ${d.added} added, ${d.skipped} skipped${
        d.errors > 0 ? `, ${d.errors} errors` : ""
      }`,
    }),
  fail: (message) => set({ scanning: false, lastError: message }),
}));
