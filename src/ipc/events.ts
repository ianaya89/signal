import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// Channel names match SignalEvent::channel() in signal-core.
export type SignalChannel =
  | "player:state"
  | "player:progress"
  | "player:track-changed"
  | "player:track-ended"
  | "player:device-changed"
  | "scanner:progress"
  | "scanner:done"
  | "scanner:error"
  | "artwork:progress"
  | "queue:changed"
  | "config:changed"
  | "log:line";

export function onSignalEvent<T>(
  channel: SignalChannel,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(channel, (event) => handler(event.payload));
}
