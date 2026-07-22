import { invoke } from "@tauri-apps/api/core";

// Grows with each milestone; keep in sync with src-tauri commands.
export type IpcCommand = "settings_get" | "settings_set";

export function ipc<T>(
  command: IpcCommand,
  args?: Record<string, unknown>,
): Promise<T> {
  return invoke<T>(command, args);
}
