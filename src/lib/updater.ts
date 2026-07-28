import { relaunch } from "@tauri-apps/plugin-process";
import type { Update } from "@tauri-apps/plugin-updater";
import { check } from "@tauri-apps/plugin-updater";

import { api } from "@/ipc/invoke";
import { toast } from "@/stores/toastStore";
import { useUpdateStore } from "@/stores/updateStore";

const AUTO_CHECK_KEY = "updates.auto_check";

// The Update handle carries the download target; keep it out of the store so
// zustand only ever holds plain data.
let pending: Update | null = null;

function inTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function message(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

/// False for .deb installs, where apt owns the upgrade and the plugin has
/// nothing it can rewrite.
export async function isUpdatable(): Promise<boolean> {
  if (!inTauri()) return false;
  try {
    return (await api.appInfo()).updatable;
  } catch {
    return false;
  }
}

export async function checkForUpdate({ silent = false } = {}): Promise<boolean> {
  if (!(await isUpdatable())) {
    if (!silent) toast.info("this install updates through your package manager");
    return false;
  }
  const store = useUpdateStore.getState();
  store.checking();
  try {
    const update = await check();
    if (!update) {
      pending = null;
      store.upToDate();
      if (!silent) toast.ok("signal is up to date");
      return false;
    }
    pending = update;
    store.found(update.version, update.body ?? null);
    if (!silent) toast.info(`update ${update.version} available`);
    return true;
  } catch (err) {
    pending = null;
    store.fail(message(err));
    // A silent check runs on every launch: offline, no release yet, or a
    // half-published manifest must not greet the user with an error.
    if (!silent) toast.error(`update check failed: ${message(err)}`);
    return false;
  }
}

export async function installUpdate(): Promise<void> {
  const store = useUpdateStore.getState();
  // one install at a time; a second click must not start a parallel download
  if (store.status === "downloading") return;
  // instant feedback: the click flips the UI before any await resolves
  store.progress(0, null);

  // The Update handle lives in the Rust resource table and can go stale
  // (long-running window, a check that ran in another session). Re-check
  // rather than dead-ending on "check first".
  if (!pending) {
    try {
      pending = await check();
    } catch (err) {
      store.fail(message(err));
      toast.error(`update failed: ${message(err)}`);
      return;
    }
    if (!pending) {
      store.upToDate();
      toast.info("no update available anymore — already up to date");
      return;
    }
    store.found(pending.version, pending.body ?? null);
    store.progress(0, null);
  }

  let received = 0;
  try {
    await pending.downloadAndInstall((event) => {
      if (event.event === "Started") {
        store.progress(0, event.data.contentLength ?? null);
      } else if (event.event === "Progress") {
        received += event.data.chunkLength;
        store.progress(received, useUpdateStore.getState().total);
      } else {
        store.ready();
      }
    });
    store.ready();
    toast.ok("update installed — restarting");
  } catch (err) {
    pending = null;
    store.fail(message(err));
    toast.error(`update failed: ${message(err)}`);
    return;
  }

  try {
    await relaunch();
  } catch (err) {
    // installed on disk either way; the user just has to reopen it
    store.fail(`installed, but the restart failed — quit and reopen signal (${message(err)})`);
  }
}

/** Opens the review dialog, kicking off a check when nothing is known yet. */
export function openUpdateDialog(): void {
  const store = useUpdateStore.getState();
  store.openDialog();
  if (store.status === "idle" || store.status === "error") {
    void checkForUpdate({ silent: true });
  }
}

export async function restartNow(): Promise<void> {
  try {
    await relaunch();
  } catch (err) {
    toast.error(`restart failed: ${message(err)}`);
  }
}

export async function isAutoCheckEnabled(): Promise<boolean> {
  try {
    const raw = await api.settingsGet(AUTO_CHECK_KEY);
    return raw !== "false";
  } catch {
    return true;
  }
}

export async function setAutoCheck(on: boolean): Promise<void> {
  useUpdateStore.getState().setAutoCheck(on);
  await api.settingsSet(AUTO_CHECK_KEY, String(on));
}

/// Launch-time check: opt-out, quiet on failure, and delayed so it never
/// competes with the library query burst on startup.
export function startupUpdateCheck(): void {
  if (!inTauri()) return;
  void (async () => {
    const enabled = await isAutoCheckEnabled();
    useUpdateStore.getState().setAutoCheck(enabled);
    if (!enabled || !(await isUpdatable())) return;
    setTimeout(() => void checkForUpdate({ silent: true }), 4000);
  })();
}
