import { create } from "zustand";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

interface UpdateState {
  status: UpdateStatus;
  version: string | null;
  notes: string | null;
  downloaded: number;
  total: number | null;
  error: string | null;
  autoCheck: boolean;
  setAutoCheck: (on: boolean) => void;
  checking: () => void;
  found: (version: string, notes: string | null) => void;
  upToDate: () => void;
  progress: (downloaded: number, total: number | null) => void;
  ready: () => void;
  fail: (message: string) => void;
}

export const useUpdateStore = create<UpdateState>((set) => ({
  status: "idle",
  version: null,
  notes: null,
  downloaded: 0,
  total: null,
  error: null,
  autoCheck: true,
  setAutoCheck: (on) => set({ autoCheck: on }),
  checking: () => set({ status: "checking", error: null }),
  found: (version, notes) => set({ status: "available", version, notes }),
  upToDate: () => set({ status: "idle", version: null, notes: null }),
  progress: (downloaded, total) => set({ status: "downloading", downloaded, total }),
  ready: () => set({ status: "ready" }),
  fail: (message) => set({ status: "error", error: message }),
}));
