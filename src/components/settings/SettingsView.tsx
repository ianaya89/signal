import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { RemoteSource, ReplayGainMode } from "@/ipc/types";
import { pickFolder, pickSavePath } from "@/lib/pickFolder";
import { checkForUpdate, openUpdateDialog, setAutoCheck } from "@/lib/updater";
import { cn, errText } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useScanStore } from "@/stores/scanStore";
import { toast } from "@/stores/toastStore";
import { useUiStore } from "@/stores/uiStore";
import { useUpdateStore } from "@/stores/updateStore";

const RG_MODES: ReplayGainMode[] = ["off", "track", "album"];

export function SettingsView() {
  useMainTitle("settings");
  const queryClient = useQueryClient();
  const theme = useUiStore((s) => s.theme);
  const toggleTheme = useUiStore((s) => s.toggleTheme);
  const { replaygain, exclusive, deviceId } = usePlayerStore();

  const { data: info } = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });
  const { data: roots } = useQuery({ queryKey: ["roots"], queryFn: api.listRoots });
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
        <Row label="folders">
          <div className="flex min-w-0 flex-1 flex-col gap-1">
            {(roots ?? []).map((root) => (
              <span key={root} className="flex items-center gap-2">
                <span className="min-w-0 flex-1 truncate text-[11px] text-secondary">
                  {root}
                </span>
                <button
                  type="button"
                  onClick={() => {
                    void api.removeRoot(root, true).then((removed) => {
                      toast.ok(`root removed · ${removed} tracks dropped (files stay)`);
                      return queryClient.invalidateQueries();
                    });
                  }}
                  title="remove this folder and its tracks from the library (files stay on disk)"
                  className={cn(BTN, "hover:border-error hover:text-error")}
                >
                  remove
                </button>
              </span>
            ))}
            {(roots ?? []).length === 0 && (
              <span className="text-[11px] text-muted">no folders yet</span>
            )}
          </div>
          <button type="button" onClick={() => void changeFolder()} className={BTN}>
            add…
          </button>
        </Row>
        <Row label="tracks">
          <span className="text-[11px] text-secondary">{info?.trackCount ?? "—"}</span>
          <button
            type="button"
            onClick={() => {
              useScanStore.getState().start();
              void api.rescanAll().catch((e) => toast.error(String(e)));
            }}
            className={BTN}
          >
            rescan all
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

      <ServerSection />

      <RemoteSourcesSection />

      <Section title="about">
        <Row label="version">
          <span className="text-[11px] text-secondary">signal v{info?.version}</span>
        </Row>
        <UpdateRows updatable={info?.updatable ?? false} />
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

function UpdateRows({ updatable }: { updatable: boolean }) {
  const { status, version, autoCheck, error, downloaded, total } = useUpdateStore();
  const pct = total ? Math.round((downloaded / total) * 100) : null;

  if (!updatable) {
    return (
      <Row label="updates">
        <span className="text-[11px] text-muted">
          handled by your package manager
        </span>
      </Row>
    );
  }

  return (
    <>
      <Row label="updates">
        <span
          className={cn(
            "min-w-0 flex-1 truncate text-[11px]",
            status === "error" ? "text-error" : "text-secondary",
          )}
        >
          {status === "checking" && "checking…"}
          {status === "available" && `v${version} available`}
          {status === "downloading" && `downloading${pct === null ? "…" : ` ${pct}%`}`}
          {status === "ready" && "installed — restart to apply"}
          {status === "error" && error}
          {status === "idle" && "up to date"}
        </span>
        {status === "available" || status === "downloading" || status === "ready" ? (
          <button
            type="button"
            onClick={openUpdateDialog}
            className={cn(BTN, "text-accent")}
          >
            {status === "ready" ? "restart to apply" : "review + install"}
          </button>
        ) : (
          <button
            type="button"
            onClick={() => void checkForUpdate()}
            disabled={status === "checking"}
            className={cn(BTN, "disabled:opacity-50")}
          >
            check now
          </button>
        )}
      </Row>
      <Row label="on launch">
        <button
          type="button"
          onClick={() => void setAutoCheck(!autoCheck)}
          className={cn(BTN, autoCheck ? "bg-raised text-accent" : undefined)}
        >
          {autoCheck ? "check automatically" : "never check"}
        </button>
      </Row>
    </>
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
      toast.error(errText(err));
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

function ServerSection() {
  const queryClient = useQueryClient();
  const [port, setPort] = useState("");
  const [password, setPassword] = useState("");
  const { data: status } = useQuery({
    queryKey: ["server-status"],
    queryFn: api.serverStatus,
  });

  // start is only enabled once a password exists (stored or typed)
  const canStart = (status?.hasPassword ?? false) || password.length > 0;

  const savePending = async () => {
    if (port) {
      const n = Number(port);
      if (n < 1 || n > 65_535) {
        throw new Error("port must be between 1 and 65535");
      }
      await api.settingsSet("server.port", port);
    }
    if (password) await api.settingsSet("server.password", password);
  };

  const saveConfig = async () => {
    try {
      await savePending();
      toast.ok("server settings saved — restart the server to apply");
      setPassword("");
      void queryClient.invalidateQueries({ queryKey: ["server-status"] });
    } catch (err) {
      toast.error(errText(err));
    }
  };

  const toggle = async () => {
    try {
      if (status?.running) {
        await api.serverStop();
        toast.ok("server stopped");
      } else {
        // unsaved port/password in the inputs count — starting implies them
        await savePending();
        setPassword("");
        const started = await api.serverStart();
        toast.ok(`serving on port ${started.port}`);
      }
      void queryClient.invalidateQueries({ queryKey: ["server-status"] });
    } catch (err) {
      toast.error(errText(err));
    }
  };

  return (
    <Section title="mobile server">
      <Row label="opensubsonic">
        <span
          className={cn("text-[11px]", status?.running ? "text-ok" : "text-muted")}
        >
          {status?.running
            ? `● http://${status.lanIp ?? "<this-machine's-ip>"}:${status.port}`
            : "○ off"}
        </span>
        <button
          type="button"
          onClick={() => void toggle()}
          disabled={!status?.running && !canStart}
          title={
            !status?.running && !canStart
              ? "set a password below first — subsonic clients require one"
              : undefined
          }
          className={cn(BTN, "disabled:cursor-not-allowed disabled:opacity-40")}
        >
          {status?.running ? "stop" : "start"}
        </button>
        {!status?.running && !canStart && (
          <span className="text-[10px] text-warn">needs a password</span>
        )}
      </Row>
      <Row label="port">
        <input
          value={port}
          onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))}
          placeholder={String(status?.port ?? 4040)}
          inputMode="numeric"
          spellCheck={false}
          className="w-24 border border-subtle bg-base/60 px-2 py-0.5 text-[11px] text-primary outline-none focus:border-focus"
        />
      </Row>
      <Row label="password">
        <input
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="required before starting"
          spellCheck={false}
          type="password"
          className="w-72 border border-subtle bg-base/60 px-2 py-0.5 text-[11px] text-primary outline-none focus:border-focus"
        />
        <button type="button" onClick={() => void saveConfig()} className={BTN}>
          save
        </button>
      </Row>
      <p className="text-[10px] text-muted">
        point symfonium/amperfy/feishin at the address above — any username,
        this password. LAN only, no transcoding.
      </p>
    </Section>
  );
}

function RemoteSourcesSection() {
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [insecure, setInsecure] = useState(false);
  const [testing, setTesting] = useState<number | null>(null);

  const { data: sources } = useQuery({
    queryKey: ["remote-sources"],
    queryFn: api.remoteSourceList,
  });

  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: ["remote-sources"] });

  const add = async () => {
    try {
      const added = await api.remoteSourceAdd(
        name || baseUrl,
        baseUrl,
        username,
        password,
        insecure,
      );
      setName("");
      setBaseUrl("");
      setUsername("");
      setPassword("");
      setInsecure(false);
      await refresh();
      // probe straight away — a typo'd host or password is worth knowing now
      const status = await api.remoteTestConnection(added.id);
      await refresh();
      if (status.ok) {
        toast.ok(
          `${added.name} connected · ${status.serverType ?? "subsonic"} ${status.serverVersion ?? ""}`.trim(),
        );
      } else {
        toast.error(`${added.name}: ${status.error ?? "unreachable"}`);
      }
    } catch (err) {
      toast.error(errText(err));
    }
  };

  const test = async (source: RemoteSource) => {
    setTesting(source.id);
    try {
      const status = await api.remoteTestConnection(source.id);
      await refresh();
      if (status.ok) {
        toast.ok(`${source.name} reachable · ${status.authMode} auth`);
      } else {
        toast.error(`${source.name}: ${status.error ?? "unreachable"}`);
      }
    } catch (err) {
      toast.error(errText(err));
    } finally {
      setTesting(null);
    }
  };

  const remove = async (source: RemoteSource) => {
    try {
      await api.remoteSourceRemove(source.id);
      await refresh();
      toast.ok(`${source.name} removed`);
    } catch (err) {
      toast.error(errText(err));
    }
  };

  const toggleInsecure = async (source: RemoteSource) => {
    try {
      await api.remoteSourceUpdate(source.id, {
        allowInsecureTls: !source.allowInsecureTls,
      });
      await refresh();
    } catch (err) {
      toast.error(errText(err));
    }
  };

  return (
    <Section title="remote servers">
      {(sources ?? []).map((source) => (
        <div key={source.id} className="flex flex-col gap-1">
          <Row label={source.name}>
            <span
              className={cn(
                "text-[11px]",
                source.lastPingOk === true
                  ? "text-ok"
                  : source.lastPingOk === false
                    ? "text-error"
                    : "text-muted",
              )}
            >
              {source.lastPingOk === true
                ? "● connected"
                : source.lastPingOk === false
                  ? "● unreachable"
                  : "○ untested"}
            </span>
            <button
              type="button"
              onClick={() => void test(source)}
              disabled={testing === source.id}
              className={cn(BTN, "disabled:opacity-50")}
            >
              {testing === source.id ? "testing…" : "test"}
            </button>
            <button
              type="button"
              onClick={() => void remove(source)}
              className={cn(BTN, "hover:border-error hover:text-error")}
            >
              remove
            </button>
          </Row>
          <div className="flex items-center gap-3 pl-[7.75rem]">
            <span className="min-w-0 flex-1 truncate text-[10px] text-muted">
              {source.baseUrl} · {source.username} · {source.authMode} auth
            </span>
            <button
              type="button"
              onClick={() => void toggleInsecure(source)}
              title="accept self-signed certificates from this server"
              className={cn(
                "px-1 text-[10px]",
                source.allowInsecureTls
                  ? "text-warn"
                  : "text-muted hover:text-secondary",
              )}
            >
              {source.allowInsecureTls ? "tls: unverified" : "tls: verified"}
            </button>
          </div>
        </div>
      ))}
      {(sources ?? []).length === 0 && (
        <p className="text-[11px] text-muted">no remote servers yet</p>
      )}

      <Row label="name">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="optional — defaults to the url"
          spellCheck={false}
          className={INPUT}
        />
      </Row>
      <Row label="url">
        <input
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
          placeholder="https://music.example.com"
          spellCheck={false}
          className={INPUT}
        />
      </Row>
      <Row label="username">
        <input
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          spellCheck={false}
          className={INPUT}
        />
      </Row>
      <Row label="password">
        <input
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          type="password"
          spellCheck={false}
          className={INPUT}
        />
        <button
          type="button"
          onClick={() => void add()}
          disabled={!baseUrl || !username || !password}
          className={cn(BTN, "disabled:cursor-not-allowed disabled:opacity-40")}
        >
          add
        </button>
      </Row>
      <Row label="tls">
        <button
          type="button"
          onClick={() => setInsecure(!insecure)}
          className={cn(BTN, insecure ? "text-warn" : undefined)}
        >
          {insecure ? "accept self-signed" : "verify certificates"}
        </button>
      </Row>
      <p className="text-[10px] text-muted">
        browse and stream from navidrome/airsonic/gonic. nothing is copied into
        the local library — remote tracks stream on demand and can't be staged
        in the queue.
      </p>
    </Section>
  );
}

const INPUT =
  "w-72 border border-subtle bg-base/60 px-2 py-0.5 text-[11px] text-primary outline-none focus:border-focus";

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
