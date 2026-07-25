import { invoke } from "@tauri-apps/api/core";
import {
  currentMonitor,
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
  PhysicalSize,
} from "@tauri-apps/api/window";

import { useUiStore, type WindowMode } from "@/stores/uiStore";

const SIZES: Record<Exclude<WindowMode, "full">, [number, number]> = {
  mini: [440, 118],
  dot: [76, 76],
};

let savedSize: PhysicalSize | null = null;
let savedPosition: PhysicalPosition | null = null;

/** Switches between full / mini / dot window shapes; remembers the full
 *  window's size AND position across the compact modes, clamping back
 *  on-screen if the restore would land outside the monitor. */
export async function setWindowMode(next: WindowMode) {
  const win = getCurrentWindow();
  const prev = useUiStore.getState().windowMode;
  if (prev === next) return;

  if (prev === "full") {
    savedSize = await win.innerSize();
    savedPosition = await win.outerPosition();
  }

  // hide the native chrome (traffic lights) while compact
  await invoke("window_set_compact", { compact: next !== "full" }).catch(() => {});

  if (next === "full") {
    await win.setAlwaysOnTop(false);
    if (savedSize) {
      await win.setSize(savedSize);
    } else {
      await win.setSize(new LogicalSize(1280, 800));
    }
    if (savedPosition) {
      await win.setPosition(savedPosition);
    }
    await clampOnScreen();
    savedSize = null;
    savedPosition = null;
  } else {
    const [w, h] = SIZES[next];
    await win.setAlwaysOnTop(true);
    await win.setSize(new LogicalSize(w, h));
    // the compact window keeps its dragged spot, but never off-screen
    await clampOnScreen();
  }

  useUiStore.getState().setWindowMode(next);
}

/** Centers the window when it does not meaningfully overlap its monitor. */
async function clampOnScreen() {
  const win = getCurrentWindow();
  const monitor = await currentMonitor();
  if (!monitor) return;

  const pos = await win.outerPosition();
  const size = await win.outerSize();
  const m = monitor.position;
  const ms = monitor.size;

  const MARGIN = 40; // px of the window that must stay reachable
  const offLeft = pos.x + size.width < m.x + MARGIN;
  const offRight = pos.x > m.x + ms.width - MARGIN;
  const offTop = pos.y < m.y; // titlebar above the screen is unreachable
  const offBottom = pos.y > m.y + ms.height - MARGIN;

  if (offLeft || offRight || offTop || offBottom) {
    await win.center();
  }
}
