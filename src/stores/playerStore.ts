import { create } from "zustand";

import type { ReplayGainMode } from "@/ipc/types";

export type PlaybackStatus = "stopped" | "playing" | "paused";

export interface PlayerStateDto {
  status: PlaybackStatus;
  trackId: number | null;
  positionMs: number;
  durationMs: number;
  volume: number;
  deviceId: string | null;
  replaygain: ReplayGainMode;
  exclusive: boolean;
  bitPerfect: boolean;
  sourceRateHz: number | null;
  outputRateHz: number | null;
}

interface PlayerStore extends PlayerStateDto {
  applyState: (s: PlayerStateDto) => void;
  applyProgress: (positionMs: number, durationMs: number) => void;
}

export const usePlayerStore = create<PlayerStore>((set) => ({
  status: "stopped",
  trackId: null,
  positionMs: 0,
  durationMs: 0,
  volume: 1,
  deviceId: null,
  replaygain: "off",
  exclusive: false,
  bitPerfect: false,
  sourceRateHz: null,
  outputRateHz: null,
  applyState: (s) => set(s),
  applyProgress: (positionMs, durationMs) =>
    set(durationMs > 0 ? { positionMs, durationMs } : { positionMs }),
}));
