import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { ReplayGainMode } from "@/ipc/types";
import { pickFolder, pickSavePath } from "@/lib/pickFolder";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useScanStore } from "@/stores/scanStore";
import { toast } from "@/stores/toastStore";
import { useUiStore } from "@/stores/uiStore";

const RG_MODES: ReplayGainMode[] = ["off", "track", "album"];

export function SettingsView() {
  useMainTitle("settings");
  const queryClient = useQueryClient();
  const theme = useUiStore((s) => s.theme);
  const toggleTheme = useUiStore((s) => s.toggleTheme);
  const { replaygain, exclusive, deviceId } = usePlayerStore();

  const { data: info } = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });
  const { data: devices } = useQuery({
    queryKey: ["devices"],
    queryFn: api.deviceList,
    staleTime: 30_000,
  });

  const changeFolder = async () => {
    const folder = await pickFolder();
    if (!folder) return;
    useScanStore.getState().start();
    await api.scanLibrary(folder);
    void queryClient.invalidateQueries({ queryKey: ["app-info"] });
  };

  return (
    <div className="flex max-w-xl flex-col gap-5 p-4">
      <Section title="library">
        <Row label="root folder">
          <span className="min-w-0 flex-1 truncate text-[11px] text-secondary">
            {info?.libraryRoot ?? "not set"}
          </span>
          <button type="button" onClick={() => void changeFolder()} className={BTN}>
            change…
          </button>
        </Row>
        <Row label="tracks">
          <span className="text-[11px] text-secondary">{info?.trackCount ?? "—"}</span>
          <button
            type="button"
            onClick={() => {
              useScanStore.getState().start();
              void api
                .settingsGet("library.root")
                .then((root) => (root ? api.scanLibrary(root) : undefined));
            }}
            className={BTN}
          >
            rescan
          </button>
          <button
            type="button"
            onClick={() => {
              useScanStore.getState().start();
              void api.resetAndRescan().catch((e) => toast.error(String(e)));
            }}
            className={cn(BTN, "hover:border-error hover:text-error")}
          >
            reset + rescan
          </button>
        </Row>
      </Section>

      <Section title="playback">
        <Row label="output device">
          <select
            value={deviceId ?? "auto"}
            onChange={(e) => void api.deviceSelect(e.target.value)}
            className="w-56 truncate border border-subtle bg-base/60 px-1 py-0.5 text-[11px] text-secondary outline-none focus:border-focus"
          >
            {!devices?.some((d) => d.id === (deviceId ?? "auto")) && (
              <option value={deviceId ?? "auto"}>auto</option>
            )}
            {devices?.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>
        </Row>
        <Row label="replaygain">
          <div className="flex gap-px overflow-hidden border border-subtle">
            {RG_MODES.map((mode) => (
              <button
                key={mode}
                type="button"
                onClick={() => void api.setReplaygain(mode)}
                className={cn(
                  "px-2 py-0.5 text-[11px]",
                  replaygain === mode
                    ? "bg-raised text-accent"
                    : "text-muted hover:text-secondary",
                )}
              >
                {mode}
              </button>
            ))}
          </div>
        </Row>
        <Row label="exclusive mode">
          <button
            type="button"
            onClick={() => void api.setExclusive(!exclusive)}
            className={cn(
              BTN,
              exclusive ? "bg-raised text-accent" : undefined,
            )}
          >
            {exclusive ? "on" : "off"}
          </button>
        </Row>
      </Section>

      <Section title="appearance">
        <Row label="theme">
          <button type="button" onClick={toggleTheme} className={BTN}>
            {theme === "dark" ? "dark (indigo)" : "light (manila)"} — switch
          </button>
        </Row>
      </Section>

      <PluginsSection />

      <Section title="about">
        <Row label="version">
          <span className="text-[11px] text-secondary">signal v{info?.version}</span>
        </Row>
        <Row label="database">
          <button
            type="button"
            onClick={() => info && void api.revealFile(info.dbPath)}
            className="min-w-0 flex-1 truncate text-left text-[11px] text-muted hover:text-accent"
            title="reveal in finder"
          >
            {info?.dbPath}
          </button>
          <button
            type="button"
            onClick={() => {
              void (async () => {
                const stamp = new Date().toISOString().slice(0, 10);
                const dest = await pickSavePath(`signal-backup-${stamp}.db`, "db");
                if (!dest) return;
                await api.libraryBackup(dest);
                toast.ok("database backed up");
              })().catch((e) => toast.error(String(e)));
            }}
            className={BTN}
          >
            backup…
          </button>
        </Row>
      </Section>
    </div>
  );
}

function PluginsSection() {
  const queryClient = useQueryClient();
  const [token, setToken] = useState("");
  const { data: status } = useQuery({
    queryKey: ["plugin-status"],
    queryFn: api.pluginStatus,
  });

  const save = async () => {
    try {
      const valid = await api.setListenBrainz(token);
      toast.ok(valid ? "listenbrainz connected" : "listenbrainz disabled");
      setToken("");
      void queryClient.invalidateQueries({ queryKey: ["plugin-status"] });
    } catch (err) {
      toast.error(String(err));
    }
  };

  return (
    <Section title="scrobbling">
      <Row label="listenbrainz">
        <span
          className={cn(
            "text-[11px]",
            status?.listenbrainz ? "text-ok" : "text-muted",
          )}
        >
          {status?.listenbrainz ? "● connected" : "○ off"}
        </span>
      </Row>
      <Row label="user token">
        <input
          value={token}
          onChange={(e) => setToken(e.target.value)}
          placeholder="paste token from listenbrainz.org/settings"
          spellCheck={false}
          type="password"
          className="w-72 border border-subtle bg-base/60 px-2 py-0.5 text-[11px] text-primary outline-none focus:border-focus"
        />
        <button type="button" onClick={() => void save()} className={BTN}>
          save
        </button>
      </Row>
      <p className="text-[10px] text-muted">
        completed listens (≥50% or 4min) are submitted; empty token disables
      </p>
    </Section>
  );
}

const BTN =
  "border border-subtle bg-raised px-2 py-0.5 text-[11px] text-secondary hover:border-focus hover:text-accent";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section>
      <h2 className="mb-2 text-[10px] uppercase tracking-wider text-muted">
        {title}
      </h2>
      <div className="flex flex-col gap-2">{children}</div>
    </section>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3">
      <span className="w-28 shrink-0 text-[11px] text-muted">{label}</span>
      {children}
    </div>
  );
}
