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
const TAB_KEY = "settings.tab";

/** Tab order is the order you meet these settings: what's in the library,
 *  how it sounds, how it looks, then the things that talk to the network. */
const TABS = [
  { key: "library", blurb: "where your music lives on disk" },
  { key: "playback", blurb: "how audio reaches your speakers" },
  { key: "appearance", blurb: "how signal looks" },
  { key: "scrobbling", blurb: "report what you listen to" },
  {
    key: "server",
    blurb: "serve this library over OpenSubsonic, to players on your network",
  },
  {
    key: "remote",
    blurb: "stream from someone else's OpenSubsonic server",
  },
  { key: "about", blurb: "version, updates, database" },
] as const;

type TabKey = (typeof TABS)[number]["key"];

export function SettingsView() {
  useMainTitle("settings");
  const [tab, setTab] = useState<TabKey>(() => {
    const saved = localStorage.getItem(TAB_KEY) as TabKey | null;
    return TABS.some((t) => t.key === saved) && saved ? saved : "library";
  });

  const active = TABS.find((t) => t.key === tab) ?? TABS[0];

  const select = (key: TabKey) => {
    setTab(key);
    localStorage.setItem(TAB_KEY, key);
  };

  // a tablist is expected to move with the arrow keys, not just Tab
  const onTabKeyDown = (e: React.KeyboardEvent) => {
    const delta = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (delta === 0) return;
    e.preventDefault();
    const index = TABS.findIndex((t) => t.key === tab);
    const next = TABS[(index + delta + TABS.length) % TABS.length];
    select(next.key);
    document.getElementById(`settings-tab-${next.key}`)?.focus();
  };

  return (
    <div className="flex h-full flex-col">
      <div
        role="tablist"
        aria-label="settings sections"
        onKeyDown={onTabKeyDown}
        className="flex h-7 shrink-0 items-center gap-1 overflow-x-auto border-b border-subtle px-3 text-[10px]"
      >
        {TABS.map(({ key }) => (
          <button
            key={key}
            type="button"
            role="tab"
            id={`settings-tab-${key}`}
            aria-selected={tab === key}
            aria-controls={`settings-panel-${key}`}
            tabIndex={tab === key ? 0 : -1}
            onClick={() => select(key)}
            className={cn(
              "shrink-0 px-1.5 py-0.5",
              tab === key
                ? "bg-raised text-accent"
                : "text-muted hover:text-secondary",
            )}
          >
            {key}
          </button>
        ))}
      </div>

      <div
        role="tabpanel"
        id={`settings-panel-${tab}`}
        aria-labelledby={`settings-tab-${tab}`}
        className="min-h-0 flex-1 overflow-auto"
      >
        <div className="flex max-w-xl flex-col gap-5 p-4">
          <p className="text-[11px] text-muted">{active.blurb}</p>
          {tab === "library" && <LibrarySection />}
          {tab === "playback" && <PlaybackSection />}
          {tab === "appearance" && <AppearanceSection />}
          {tab === "scrobbling" && <PluginsSection />}
          {tab === "server" && <ServerSection />}
          {tab === "remote" && <RemoteSourcesSection />}
          {tab === "about" && <AboutSection />}
        </div>
      </div>
    </div>
  );
}

function LibrarySection() {
  const queryClient = useQueryClient();
  const { data: info } = useQuery({
    queryKey: ["app-info"],
    queryFn: api.appInfo,
  });
  const { data: roots } = useQuery({
    queryKey: ["roots"],
    queryFn: api.listRoots,
  });

  const addFolder = async () => {
    const folder = await pickFolder();
    if (!folder) return;
    useScanStore.getState().start();
    await api.scanLibrary(folder);
    void queryClient.invalidateQueries({ queryKey: ["app-info"] });
  };

  return (
    <>
      <Section
        title="folders"
        hint="scanned recursively and watched for changes. use the picker rather than a typed path — on macOS that is what grants access to icloud and external drives."
      >
        <div className="flex flex-col">
          {(roots ?? []).map((root) => (
            <div
              key={root}
              className="group flex items-center gap-2 border border-subtle border-b-0 px-2 py-1 last:border-b"
            >
              <span className="min-w-0 flex-1 truncate text-[11px] text-secondary">
                {root}
              </span>
              <button
                type="button"
                onClick={() => {
                  void api.removeRoot(root, true).then((removed) => {
                    toast.ok(
                      `root removed · ${removed} tracks dropped (files stay)`,
                    );
                    return queryClient.invalidateQueries();
                  });
                }}
                title="remove this folder and its tracks from the library (files stay on disk)"
                className={BTN_DANGER}
              >
                remove
              </button>
            </div>
          ))}
          {(roots ?? []).length === 0 && (
            <p className="border border-subtle px-2 py-1 text-[11px] text-muted">
              no folders yet — nothing will show up in the library until you add
              one
            </p>
          )}
        </div>
        <button type="button" onClick={() => void addFolder()} className={BTN}>
          add folder…
        </button>
      </Section>

      <Section title="maintenance">
        <Row label="tracks">
          <span className="text-[11px] text-secondary">
            {info?.trackCount ?? "—"}
          </span>
        </Row>
        <Row
          label="rescan"
          hint="walks the folders above again and imports anything new. only touches folders already in the list."
        >
          <button
            type="button"
            onClick={() => {
              useScanStore.getState().start();
              void api.rescanAll().catch((e) => toast.error(errText(e)));
            }}
            className={BTN}
          >
            rescan all
          </button>
        </Row>
        <Row
          label="rebuild"
          hint="drops every track from the database and imports from scratch. ratings, play counts and playlists are lost. audio files are untouched."
        >
          <button
            type="button"
            onClick={() => {
              useScanStore.getState().start();
              void api.resetAndRescan().catch((e) => toast.error(errText(e)));
            }}
            className={BTN_DANGER}
          >
            reset + rescan
          </button>
        </Row>
      </Section>
    </>
  );
}

function PlaybackSection() {
  const { replaygain, exclusive, deviceId } = usePlayerStore();
  const { data: devices } = useQuery({
    queryKey: ["devices"],
    queryFn: api.deviceList,
    staleTime: 30_000,
  });

  return (
    <Section title="output">
      <Row label="device">
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
      <Row
        label="replaygain"
        hint="levels volume across tracks using tags written by your ripper. track evens out a shuffle; album preserves the dynamics within a record."
      >
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
      <Row
        label="exclusive"
        hint="takes sole control of the audio device for bit-perfect output. other apps lose sound while signal plays."
      >
        <Toggle
          on={exclusive}
          onClick={() => void api.setExclusive(!exclusive)}
        />
      </Row>
    </Section>
  );
}

function AppearanceSection() {
  const theme = useUiStore((s) => s.theme);
  const toggleTheme = useUiStore((s) => s.toggleTheme);

  return (
    <Section title="theme">
      <Row label="palette">
        <button type="button" onClick={toggleTheme} className={BTN}>
          {theme === "dark" ? "dark (indigo)" : "light (manila)"} — switch
        </button>
      </Row>
    </Section>
  );
}

function AboutSection() {
  const { data: info } = useQuery({
    queryKey: ["app-info"],
    queryFn: api.appInfo,
  });

  const backup = () => {
    void (async () => {
      const stamp = new Date().toISOString().slice(0, 10);
      const dest = await pickSavePath(`signal-backup-${stamp}.db`, "db");
      if (!dest) return;
      await api.libraryBackup(dest);
      toast.ok("database backed up");
    })().catch((e) => toast.error(errText(e)));
  };

  return (
    <>
      <Section title="version">
        <Row label="signal">
          <span className="text-[11px] text-secondary">v{info?.version}</span>
        </Row>
        <UpdateRows updatable={info?.updatable ?? false} />
      </Section>

      <Section
        title="database"
        hint="ratings, play counts and playlists live here — worth backing up before a reset + rescan."
      >
        <div className="flex items-center gap-2 border border-subtle px-2 py-1">
          <button
            type="button"
            onClick={() => info && void api.revealFile(info.dbPath)}
            className="min-w-0 flex-1 truncate text-left text-[11px] text-muted hover:text-accent"
            title="reveal in finder"
          >
            {info?.dbPath}
          </button>
          <button type="button" onClick={backup} className={BTN}>
            backup…
          </button>
        </div>
      </Section>
    </>
  );
}

function UpdateRows({ updatable }: { updatable: boolean }) {
  const { status, version, autoCheck, error, downloaded, total } =
    useUpdateStore();
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
          {status === "downloading" &&
            `downloading${pct === null ? "…" : ` ${pct}%`}`}
          {status === "ready" && "installed — restart to apply"}
          {status === "error" && error}
          {status === "idle" && "up to date"}
        </span>
        {status === "available" ||
        status === "downloading" ||
        status === "ready" ? (
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
        <Toggle
          on={autoCheck}
          onClick={() => void setAutoCheck(!autoCheck)}
          labels={["check automatically", "never check"]}
        />
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
    <Section
      title="listenbrainz"
      hint="completed listens (≥50% or 4 min) are submitted. saving an empty token disables it."
    >
      <Row label="status">
        <StatusDot
          state={status?.listenbrainz ? "ok" : "off"}
          label={status?.listenbrainz ? "connected" : "off"}
        />
      </Row>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        <Row label="user token">
          <input
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="paste from listenbrainz.org/settings"
            spellCheck={false}
            type="password"
            className={INPUT}
          />
          <button type="submit" className={BTN}>
            save
          </button>
        </Row>
      </form>
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
    <Section
      title="opensubsonic"
      hint="speaks the subsonic api (1.16.1) with the opensubsonic extensions, so any subsonic client works — symfonium, amperfy, feishin. point one at the address above: any username, this password. LAN only, no transcoding."
    >
      <Row label="status">
        <StatusDot
          state={status?.running ? "ok" : "off"}
          label={
            status?.running
              ? `http://${status.lanIp ?? "<this-machine's-ip>"}:${status.port}`
              : "stopped"
          }
        />
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
      </Row>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          void saveConfig();
        }}
        className="flex flex-col gap-2"
      >
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
        <Row
          label="password"
          hint={
            !status?.running && !canStart
              ? "required before the server can start"
              : undefined
          }
        >
          <input
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder={
              status?.hasPassword ? "unchanged — type to replace" : "required"
            }
            spellCheck={false}
            type="password"
            className={INPUT}
          />
          <button type="submit" className={BTN}>
            save
          </button>
        </Row>
      </form>
    </Section>
  );
}

function RemoteSourcesSection() {
  const queryClient = useQueryClient();
  const [adding, setAdding] = useState(false);
  const [testing, setTesting] = useState<number | null>(null);

  const { data: sources } = useQuery({
    queryKey: ["remote-sources"],
    queryFn: api.remoteSourceList,
  });

  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: ["remote-sources"] });

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
    <Section
      title="servers"
      hint="anything that speaks the subsonic api (1.16.1) works — navidrome, airsonic, gonic, or another copy of signal with its own server switched on. nothing is copied into the local library: remote tracks stream on demand, so they can't be staged in the queue."
    >
      <div className="flex flex-col gap-2">
        {(sources ?? []).map((source) => (
          <ServerCard
            key={source.id}
            source={source}
            testing={testing === source.id}
            onTest={() => void test(source)}
            onRemove={() => void remove(source)}
            onToggleTls={() => void toggleInsecure(source)}
          />
        ))}
        {(sources ?? []).length === 0 && !adding && (
          <p className="border border-subtle px-2 py-1 text-[11px] text-muted">
            no servers yet — add a subsonic address to browse it from the{" "}
            <span className="text-secondary">remote</span> sidebar entry
          </p>
        )}
      </div>

      {adding ? (
        <AddServerForm
          onCancel={() => setAdding(false)}
          onAdded={() => {
            setAdding(false);
            void refresh();
          }}
          onRefresh={() => void refresh()}
        />
      ) : (
        <button
          type="button"
          onClick={() => setAdding(true)}
          className={cn(BTN, "self-start")}
        >
          + add server
        </button>
      )}
    </Section>
  );
}

function ServerCard({
  source,
  testing,
  onTest,
  onRemove,
  onToggleTls,
}: {
  source: RemoteSource;
  testing: boolean;
  onTest: () => void;
  onRemove: () => void;
  onToggleTls: () => void;
}) {
  const checked = fmtAgo(source.lastPingAt);

  return (
    <div className="border border-subtle">
      <div className="flex items-center gap-2 border-b border-subtle px-2 py-1">
        <StatusDot
          state={
            source.lastPingOk === true
              ? "ok"
              : source.lastPingOk === false
                ? "error"
                : "off"
          }
          label={
            source.lastPingOk === true
              ? "connected"
              : source.lastPingOk === false
                ? "unreachable"
                : "untested"
          }
        />
        <span className="min-w-0 flex-1 truncate text-[12px] text-primary">
          {source.name}
        </span>
        <button
          type="button"
          onClick={onTest}
          disabled={testing}
          className={cn(BTN, "disabled:opacity-50")}
        >
          {testing ? "testing…" : "test"}
        </button>
        <button type="button" onClick={onRemove} className={BTN_DANGER}>
          remove
        </button>
      </div>
      <dl className="flex flex-col gap-0.5 px-2 py-1.5">
        <Meta term="url" value={source.baseUrl} />
        <Meta term="user" value={source.username} />
        <Meta
          term="auth"
          value={`${source.authMode === "token" ? "token" : "plaintext"}${
            checked ? ` · checked ${checked}` : ""
          }`}
        />
        <div className="flex gap-2 text-[10px]">
          <dt className="w-10 shrink-0 text-muted">tls</dt>
          <dd className="min-w-0 flex-1">
            <button
              type="button"
              onClick={onToggleTls}
              title="accept self-signed certificates from this server"
              className={cn(
                "hover:underline",
                source.allowInsecureTls ? "text-warn" : "text-secondary",
              )}
            >
              {source.allowInsecureTls
                ? "certificates not verified"
                : "certificates verified"}
            </button>
          </dd>
        </div>
      </dl>
    </div>
  );
}

function AddServerForm({
  onAdded,
  onRefresh,
  onCancel,
}: {
  onAdded: () => void;
  onRefresh: () => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [insecure, setInsecure] = useState(false);
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    setBusy(true);
    try {
      const added = await api.remoteSourceAdd(
        name || baseUrl,
        baseUrl,
        username,
        password,
        insecure,
      );
      // close the form as soon as the row exists, so the probe below happens
      // against a card the user can already see
      onAdded();
      // probe straight away — a typo'd host or password is worth knowing now
      const status = await api.remoteTestConnection(added.id);
      onRefresh();
      if (status.ok) {
        toast.ok(
          `${added.name} connected · ${status.serverType ?? "subsonic"} ${
            status.serverVersion ?? ""
          }`.trim(),
        );
      } else {
        toast.error(`${added.name}: ${status.error ?? "unreachable"}`);
      }
    } catch (err) {
      toast.error(errText(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
      className="flex flex-col gap-2 border border-subtle p-2"
    >
      <h4 className="text-[10px] uppercase tracking-wider text-muted">
        new server
      </h4>
      <Row label="url">
        <input
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
          placeholder="https://music.example.com"
          spellCheck={false}
          autoFocus
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
      </Row>
      <Row label="name" hint="optional — defaults to the url">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          spellCheck={false}
          className={INPUT}
        />
      </Row>
      <Row
        label="tls"
        hint={
          insecure
            ? "any certificate is accepted — only do this on a network you trust"
            : undefined
        }
      >
        <Toggle
          on={insecure}
          onClick={() => setInsecure(!insecure)}
          labels={["accept self-signed", "verify certificates"]}
          danger
        />
      </Row>
      <div className={ROW_GRID}>
        <span aria-hidden />
        <div className="flex items-center gap-2">
          <button
            type="submit"
            disabled={busy || !baseUrl || !username || !password}
            className={cn(
              BTN,
              "disabled:cursor-not-allowed disabled:opacity-40",
            )}
          >
            {busy ? "connecting…" : "add + test"}
          </button>
          <button
            type="button"
            onClick={onCancel}
            className="px-1 text-[11px] text-muted hover:text-secondary"
          >
            cancel
          </button>
        </div>
      </div>
    </form>
  );
}

/** Relative time for a stored ISO timestamp; null when never set. */
function fmtAgo(iso: string | null): string | null {
  if (!iso) return null;
  const ms = Date.now() - new Date(iso).getTime();
  if (!Number.isFinite(ms) || ms < 0) return null;
  const min = Math.floor(ms / 60_000);
  if (min < 1) return "just now";
  if (min < 60) return `${min}m ago`;
  const hours = Math.floor(min / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

const INPUT =
  "w-72 border border-subtle bg-base/60 px-2 py-0.5 text-[11px] text-primary outline-none focus:border-focus";

const BTN =
  "shrink-0 border border-subtle bg-raised px-2 py-0.5 text-[11px] text-secondary hover:border-focus hover:text-accent";

// destructive actions read as destructive at rest, not only on hover
const BTN_DANGER =
  "shrink-0 border border-subtle bg-raised px-2 py-0.5 text-[11px] text-error/80 hover:border-error hover:text-error";

function StatusDot({
  state,
  label,
}: {
  state: "ok" | "error" | "off";
  label: string;
}) {
  const tone =
    state === "ok" ? "text-ok" : state === "error" ? "text-error" : "text-muted";
  return (
    <span className={cn("flex shrink-0 items-center gap-1 text-[11px]", tone)}>
      <span aria-hidden>{state === "off" ? "○" : "●"}</span>
      {label}
    </span>
  );
}

function Toggle({
  on,
  onClick,
  labels = ["on", "off"],
  danger = false,
}: {
  on: boolean;
  onClick: () => void;
  labels?: [string, string];
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      onClick={onClick}
      className={cn(
        BTN,
        on && (danger ? "text-warn" : "bg-raised text-accent"),
      )}
    >
      {on ? labels[0] : labels[1]}
    </button>
  );
}

function Meta({ term, value }: { term: string; value: string }) {
  return (
    <div className="flex gap-2 text-[10px]">
      <dt className="w-10 shrink-0 text-muted">{term}</dt>
      <dd className="min-w-0 flex-1 truncate text-secondary">{value}</dd>
    </div>
  );
}

function Section({
  title,
  hint,
  children,
}: {
  title: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <section>
      <h3 className="mb-2 text-[10px] uppercase tracking-wider text-muted">
        {title}
      </h3>
      <div className="flex flex-col gap-2">{children}</div>
      {hint && <p className="mt-2 text-[10px] text-muted">{hint}</p>}
    </section>
  );
}

/** Label + controls, with the explanation sitting under its own control
 *  instead of drifting to the end of the section.
 *
 *  Grid rather than flex + padding so the hint lines up with the control by
 *  construction — an indent hand-tuned to the label width silently drifts the
 *  moment that width changes. */
function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className={ROW_GRID}>
      <span className="text-[11px] text-muted">{label}</span>
      <div className="flex min-w-0 items-center gap-2">{children}</div>
      {hint && (
        <>
          <span aria-hidden />
          <p className="text-[10px] text-muted">{hint}</p>
        </>
      )}
    </div>
  );
}

const ROW_GRID = "grid grid-cols-[7rem_1fr] items-center gap-x-3 gap-y-1";
