import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize, PhysicalSize } from "@tauri-apps/api/window";

import { useUiStore, type WindowMode } from "@/stores/uiStore";

const SIZES: Record<Exclude<WindowMode, "full">, [number, number]> = {
  mini: [440, 118],
  dot: [76, 76],
};

let savedSize: PhysicalSize | null = null;

/** Switches between full / mini / dot window shapes; remembers the full
 *  size across the compact modes. */
export async function setWindowMode(next: WindowMode) {
  const win = getCurrentWindow();
  const prev = useUiStore.getState().windowMode;
  if (prev === next) return;

  if (prev === "full") {
    savedSize = await win.innerSize();
  }

  // hide the native chrome (traffic lights) while compact
  await invoke("window_set_compact", { compact: next !== "full" }).catch(() => {});

  if (next === "full") {
    await win.setAlwaysOnTop(false);
    if (savedSize) {
      await win.setSize(savedSize);
      savedSize = null;
    } else {
      await win.setSize(new LogicalSize(1280, 800));
    }
  } else {
    const [w, h] = SIZES[next];
    await win.setAlwaysOnTop(true);
    await win.setSize(new LogicalSize(w, h));
  }

  useUiStore.getState().setWindowMode(next);
}
