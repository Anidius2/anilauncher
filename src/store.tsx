import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { api, defaultSettings, errText, fileSrc, type AccountsView, type CrashHint, type Feed, type Instance, type ModProblem, type QuickPlay, type Settings, defaultUi, defaultEvents, defaultLink, defaultDiscord, type NavPage, type LinkWorld, type UpdateInfo } from "./api";
import { translate, type Key } from "./i18n";
import { applyTheme, themes } from "./themes";
import { seasonToday, type SeasonEvent } from "./components/Seasonal";
import { setSfxFlags, setSoundPack, setSoundsEnabled, setVolume, sfx, soundPacks, themeSoundPack, type SoundPack } from "./sounds";

export type Page = "home" | "builds" | "mods" | "skins" | "shots" | "news" | "profile" | "settings";

export interface ImportProgress {
  stage: string;
  done: number;
  total: number;
}

export interface Progress {
  stage: string;
  done: number;
  total: number;
}

interface Toast {
  id: number;
  text: string;
  kind: "info" | "error" | "ok";
  action?: { label: string; run: () => void };
}

export interface CrashInfo {
  code: number | null;
  hints: CrashHint[];
  crashReport?: string | null;
}

const MAX_LOG = 5000;

interface Store {
  /** a newer launcher version found on GitHub */
  update: UpdateInfo | null;
  setUpdate: (u: UpdateInfo | null) => void;
  updateOpen: boolean;
  openUpdate: (open: boolean) => void;
  /** build whose game remote is open (from the tray menu) */
  remoteFor: string | null;
  openRemote: (id: string | null) => void;
  /** Windows notification when the launcher is hidden or not in focus */
  notify: (title: string, body: string) => void;
  /** builds whose game is connected through AniLauncher Link, with the world they are in */
  linked: Record<string, LinkWorld | null>;
  page: Page;
  go: (p: Page) => void;
  settings: Settings;
  updateSettings: (patch: Partial<Settings>) => void;
  t: (key: Key, vars?: Record<string, string | number>) => string;
  accounts: AccountsView;
  setAccounts: (a: AccountsView) => void;
  instances: Instance[];
  reloadInstances: () => Promise<void>;
  selected: Instance | null;
  select: (id: string) => void;
  running: string[];
  preparing: Record<string, Progress>;
  play: (id: string, skipCheck?: boolean, quick?: QuickPlay | null) => Promise<void>;
  mask: (text: string) => string;
  season: SeasonEvent;
  previewSeason: (e: SeasonEvent) => void;
  modsSearch: string | null;
  searchMods: (query: string, instanceId?: string) => void;
  check: { id: string; problems: ModProblem[]; quick?: QuickPlay | null } | null;
  setCheck: (c: { id: string; problems: ModProblem[]; quick?: QuickPlay | null } | null) => void;
  feed: Feed | null;
  feedError: boolean;
  toast: (text: string, kind?: Toast["kind"], action?: Toast["action"], toGame?: boolean) => void;
  toasts: Toast[];
  logs: Record<string, string[]>;
  setLog: (id: string, lines: string[]) => void;
  crashes: Record<string, CrashInfo>;
  consoleFor: string | null;
  openConsole: (id: string | null) => void;
}

const Ctx = createContext<Store | null>(null);

export function useStore(): Store {
  const s = useContext(Ctx);
  if (!s) throw new Error("store");
  return s;
}

async function loadHeadingFont(path?: string | null) {
  const root = document.documentElement;
  if (!path) {
    root.style.setProperty("--font-heading", "'Pixelify Sans', 'Inter', sans-serif");
    return;
  }
  try {
    const src = fileSrc(path);
    if (!src) return;
    const face = new FontFace("AniHeading", `url("${src}")`);
    await face.load();
    document.fonts.add(face);
    root.style.setProperty("--font-heading", "'AniHeading', 'Pixelify Sans', 'Inter', sans-serif");
  } catch {
    root.style.setProperty("--font-heading", "'Pixelify Sans', 'Inter', sans-serif");
  }
}

export function StoreProvider({ children }: { children: ReactNode }) {
  const [page, setPage] = useState<Page>("home");
  const [settings, setSettings] = useState<Settings>(defaultSettings);
  const [loaded, setLoaded] = useState(false);
  const [accounts, setAccounts] = useState<AccountsView>({ accounts: [], hasLicense: false, clientIdSet: false });
  const [instances, setInstances] = useState<Instance[]>([]);
  const [running, setRunning] = useState<string[]>([]);
  const [preparing, setPreparing] = useState<Record<string, Progress>>({});
  const [feed, setFeed] = useState<Feed | null>(null);
  const [feedError, setFeedError] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [logs, setLogs] = useState<Record<string, string[]>>({});
  const [crashes, setCrashes] = useState<Record<string, CrashInfo>>({});
  const [consoleFor, setConsoleFor] = useState<string | null>(null);
  const [check, setCheck] = useState<{ id: string; problems: ModProblem[]; quick?: QuickPlay | null } | null>(null);
  const [modsSearch, setModsSearch] = useState<string | null>(null);
  const [linked, setLinked] = useState<Record<string, LinkWorld | null>>({});
  const [remoteFor, setRemoteFor] = useState<string | null>(null);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [updateOpen, openUpdate] = useState(false);
  const linkedRef = useRef(linked);
  linkedRef.current = linked;
  const launchProblems = useRef<Record<string, ModProblem[]>>({});
  const gameStarts = useRef<Record<string, number>>({});
  const launcherStart = useRef(Math.floor(Date.now() / 1000));
  const [seasonPreview, setSeasonPreview] = useState<SeasonEvent>(null);
  const previewTimer = useRef<number | undefined>(undefined);
  const previewSeason = useCallback((e: SeasonEvent) => {
    setSeasonPreview(e);
    window.clearTimeout(previewTimer.current);
    if (e) previewTimer.current = window.setTimeout(() => setSeasonPreview(null), 15000);
  }, []);
  const setLog = useCallback((id: string, lines: string[]) => setLogs((m) => ({ ...m, [id]: lines.slice(-MAX_LOG) })), []);
  const toastId = useRef(0);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
  const instancesRef = useRef<Instance[]>([]);

  const t = useCallback(
    (key: Key, vars?: Record<string, string | number>) => translate(settings.lang, key, vars),
    [settings.lang],
  );

  const toast = useCallback((text: string, kind: Toast["kind"] = "info", action?: Toast["action"], toGame = true) => {
    const id = ++toastId.current;
    const st = settingsRef.current;
    if (toGame && st.companion && st.link?.mirrorToasts)
      for (const inst of Object.keys(linkedRef.current)) void api.linkSend(inst, { cmd: "toast", title: "AniLauncher", text }).catch(() => {});
    setToasts((ts) => [...ts, { id, text, kind, action }]);
    if (kind === "error") sfx.error();
    const base = (settingsRef.current.ui?.toastSeconds ?? 3.5) * 1000;
    setTimeout(() => setToasts((ts) => ts.filter((x) => x.id !== id)), kind === "error" || action ? Math.max(9000, base) : base);
  }, []);

  const notify = useCallback((title: string, body: string) => {
    if (!settingsRef.current.ui?.notifications) return;
    if (document.visibilityState === "visible" && document.hasFocus()) return;
    void (async () => {
      try {
        let ok = await isPermissionGranted();
        if (!ok) ok = (await requestPermission()) === "granted";
        if (ok) sendNotification({ title, body });
      } catch {
        /* notifications are a nicety */
      }
    })();
  }, []);

  const updateSettings = useCallback((patch: Partial<Settings>) => {
    setSettings((s) => {
      const next = { ...s, ...patch };
      void api.saveSettings(next).catch(() => {});
      return next;
    });
  }, []);

  const reloadInstances = useCallback(async () => {
    setInstances(await api.listInstances());
  }, []);

  // First load
  useEffect(() => {
    (async () => {
      try {
        const s = await api.getSettings();
        const merged: Settings = {
          ...defaultSettings,
          ...s,
          profile: { ...defaultSettings.profile, ...(s.profile ?? {}) },
          customThemes: s.customThemes ?? [],
          onLaunch: s.onLaunch ?? "keep",
          background: { ...defaultSettings.background, ...(s.background ?? {}) },
          widgets: { ...defaultSettings.widgets, ...(s.widgets ?? {}) },
          baseMods: s.baseMods ?? {},
          streamer: s.streamer ?? false,
          streamerNote: s.streamerNote ?? "",
          ui: { ...defaultUi, ...(s.ui ?? {}) },
          events: { ...defaultEvents, ...(s.events ?? {}) },
          globalJvmArgs: s.globalJvmArgs ?? "",
          companion: s.companion ?? true,
          link: { ...defaultLink, ...(s.link ?? {}) },
          discord: { ...defaultDiscord, ...(s.discord ?? {}) },
        };
        // an old "all events off" switch carries over
        if (s.seasonal === false && !s.events) merged.events = { ...defaultEvents, newyear: false, halloween: false, birthday: false };
        setSettings(merged);
        const sp = merged.ui.startPage === "last" ? merged.ui.lastPage : merged.ui.startPage;
        if (sp && sp !== "home" && !merged.ui.hiddenNav.includes(sp)) setPage(sp);
      } catch {
        /* first run */
      }
      setLoaded(true);
      try {
        setAccounts(await api.listAccounts());
        await reloadInstances();
        setRunning(await api.runningInstances());
      } catch (e) {
        toast(errText(e), "error");
      }
      try {
        setFeed(await api.getFeed());
      } catch {
        setFeedError(true);
      }
    })();
  }, [reloadInstances, toast, notify]);

  // Theme, font, sounds, animations follow settings
  useEffect(() => {
    applyTheme(settings.theme, settings.customThemes);
    const root = document.documentElement;
    const ui = settings.ui;
    if (ui.accent) {
      root.style.setProperty("--accent", ui.accent);
      root.style.setProperty("--accent-hi", lighten(ui.accent, 0.22));
      root.style.setProperty("--on-accent", luminance(ui.accent) > 0.6 ? "#111111" : "#ffffff");
    }
    if (ui.radius !== null) root.style.setProperty("--radius", `${ui.radius}px`);
  }, [settings.theme, settings.customThemes, settings.ui.accent, settings.ui.radius]);
  useEffect(() => {
    const root = document.documentElement;
    const ui = settings.ui;
    (root.style as CSSStyleDeclaration & { zoom: string }).zoom = String(ui.scale);
    root.dataset.font = ui.bodyFont;
    root.dataset.sidebar = ui.sidebar;
    root.dataset.hero = ui.heroHeight;
    root.dataset.glass = ui.glass ? "on" : "off";
    root.style.setProperty("--hero-shade", String(ui.heroShade));
  }, [settings.ui]);
  useEffect(() => {
    void loadHeadingFont(settings.fontPath);
  }, [settings.fontPath]);
  useEffect(() => {
    setSoundsEnabled(settings.sounds);
    setSfxFlags(settings.ui.hoverSounds, settings.ui.clickSounds);
    setVolume(settings.volume);
    let p = settings.soundPack as SoundPack | "theme";
    if (p === "theme" || !soundPacks.includes(p as SoundPack)) {
      const own = settings.customThemes.find((c) => c.id === settings.theme);
      const base = own ? own.base : themes.find((x) => x.id === settings.theme)?.id ?? "official";
      p = (own?.soundPack as SoundPack) ?? themeSoundPack[base] ?? "classic";
      if (!soundPacks.includes(p as SoundPack)) p = "classic";
    }
    setSoundPack(p as SoundPack);
  }, [settings.sounds, settings.volume, settings.soundPack, settings.theme, settings.customThemes, settings.ui.hoverSounds, settings.ui.clickSounds]);
  useEffect(() => {
    document.documentElement.dataset.motion = settings.animations ? "on" : "off";
    document.documentElement.dataset.streamer = settings.streamer ? "on" : "off";
    document.documentElement.lang = settings.lang;
  }, [settings.animations, settings.lang, settings.streamer]);

  // Events from the backend
  useEffect(() => {
    const offs: Array<Promise<() => void>> = [
      listen<{ instanceId: string; stage: string; done: number; total: number }>("install-progress", (e) => {
        const p = e.payload;
        setPreparing((m) => ({ ...m, [p.instanceId]: { stage: p.stage, done: p.done, total: p.total } }));
      }),
      listen<{ instanceId: string; type: string; world?: LinkWorld; path?: string }>("link", (e) => {
        const { instanceId, type } = e.payload;
        const st = settingsRef.current;
        const tr = (k: Key, v?: Record<string, string | number>) => translate(st.lang, k, v);
        const say = (text: string) => void api.linkSend(instanceId, { cmd: "toast", title: "AniLauncher", text }).catch(() => {});
        if (type === "connected") {
          setLinked((m) => ({ ...m, [instanceId]: m[instanceId] ?? null }));
          const problems = launchProblems.current[instanceId];
          delete launchProblems.current[instanceId];
          if (problems?.length && st.link.notifyProblems)
            say(tr("link.problems", { n: problems.length, list: problems.slice(0, 2).map((p) => p.modName).join(", ") }));
          const inst = instancesRef.current.find((i) => i.id === instanceId);
          if (st.link.notifyUpdates && !st.autoUpdateMods && inst && inst.loader !== "vanilla")
            void api.checkUpdates(instanceId).then((u) => u.length > 0 && say(tr("link.updates", { n: u.length }))).catch(() => {});
        } else if (type === "world") {
          setLinked((m) => ({ ...m, [instanceId]: e.payload.world?.state === "enter" ? e.payload.world : null }));
        } else if (type === "screenshot") {
          window.dispatchEvent(new CustomEvent("anilink-shot"));
          toast(tr("link.shot"), "ok", { label: tr("link.open"), run: () => setPage("shots") }, false);
        } else if (type === "disconnected") {
          setLinked((m) => {
            const n = { ...m };
            delete n[instanceId];
            return n;
          });
        }
      }),
      listen<{ instanceId: string; lines: string[] }>("game-log", (e) => {
        const { instanceId, lines } = e.payload;
        setLogs((m) => {
          const cur = m[instanceId] ?? [];
          const next = cur.length + lines.length > MAX_LOG ? [...cur, ...lines].slice(-MAX_LOG) : [...cur, ...lines];
          return { ...m, [instanceId]: next };
        });
      }),
      listen<{ instanceId: string }>("game-started", (e) => {
        gameStarts.current[e.payload.instanceId] = Math.floor(Date.now() / 1000);
        setLogs((m) => ({ ...m, [e.payload.instanceId]: [] }));
        setCrashes((m) => {
          const n = { ...m };
          delete n[e.payload.instanceId];
          return n;
        });
        setPreparing((m) => {
          const n = { ...m };
          delete n[e.payload.instanceId];
          return n;
        });
        setRunning((r) => Array.from(new Set([...r, e.payload.instanceId])));
        if (settingsRef.current.ui?.consoleOnLaunch) setConsoleFor(e.payload.instanceId);
      }),
      listen<{ instanceId: string; code: number | null; hints: CrashHint[]; crashReport?: string | null; linkDisabled?: boolean }>("game-exited", (e) => {
        const { instanceId, code, hints, crashReport } = e.payload;
        if (e.payload.linkDisabled)
          toast(
            settingsRef.current.lang === "ru"
              ? "AniLauncher Link помешал игре запуститься — для этой сборки он выключен, следующий запуск пройдёт без него."
              : "AniLauncher Link stopped the game from starting — it is now off for this build, the next start goes without it.",
            "error",
            undefined,
            false,
          );
        setRunning((r) => r.filter((x) => x !== instanceId));
        const tr = (k: Key, v?: Record<string, string | number>) => translate(settingsRef.current.lang, k, v);
        if (code !== null && code !== 0) {
          setCrashes((m) => ({ ...m, [instanceId]: { code, hints: hints ?? [], crashReport } }));
          toast(tr("toast.crashed", { code }), "error", { label: tr("toast.openConsole"), run: () => setConsoleFor(instanceId) });
          notify("AniLauncher", tr("toast.crashed", { code }));
        } else toast(tr("toast.exited"));
        void reloadInstances();
      }),
    ];
    return () => {
      offs.forEach((p) => void p.then((off) => off()));
    };
  }, [reloadInstances, toast, notify]);

  instancesRef.current = instances;
  const selected = useMemo(() => {
    if (instances.length === 0) return null;
    return instances.find((i) => i.id === settings.selectedInstance) ?? instances[0];
  }, [instances, settings.selectedInstance]);

  const select = useCallback((id: string) => updateSettings({ selectedInstance: id }), [updateSettings]);

  const play = useCallback(
    async (id: string, skipCheck = false, quick: QuickPlay | null = null) => {
      const inst = instancesRef.current.find((i) => i.id === id);
      if (!skipCheck && settingsRef.current.checkConflicts && inst && inst.loader !== "vanilla") {
        try {
          const problems = await Promise.race([
            api.checkMods(id),
            new Promise<never>((_, rej) => setTimeout(() => rej(new Error("timeout")), 15000)),
          ]);
          if (problems.length > 0) {
            sfx.error();
            launchProblems.current[id] = problems;
            setCheck({ id, problems, quick });
            return;
          }
        } catch {
          /* the check must never block playing */
        }
      }
      setCheck(null);
      if (!skipCheck) delete launchProblems.current[id];
      sfx.play();
      setPreparing((m) => ({ ...m, [id]: { stage: "meta", done: 0, total: 1 } }));
      try {
        await api.launchInstance(id, quick);
        toast(translate(settingsRef.current.lang, "toast.started"), "ok");
      } catch (e) {
        toast(errText(e), "error");
      } finally {
        setPreparing((m) => {
          const n = { ...m };
          delete n[id];
          return n;
        });
      }
    },
    [toast],
  );

  // Discord status follows the game, the build and (through AniLauncher Link) the world
  useEffect(() => {
    const d = settings.discord;
    if (!loaded) return;
    if (!d?.enabled) {
      void api.discordPresence(null).catch(() => {});
      return;
    }
    const ru = settings.lang !== "en";
    const hide = settings.streamer;
    const id = running[0];
    const inst = instances.find((i) => i.id === id);
    let presence: { details: string; state: string; start: number | null; largeText: string } | null = null;
    if (id && inst) {
      const world = linked[id];
      const where = !d.showWorld || !(id in linked)
        ? ""
        : !world
          ? ru ? " · в меню" : " · in the menu"
          : world.kind === "singleplayer"
            ? ru ? " · в одиночном мире" : " · in singleplayer"
            : world.kind === "multiplayer"
              ? ru ? " · на сервере" : " · on a server"
              : "";
      const loader = ({ fabric: "Fabric", quilt: "Quilt", forge: "Forge", neoforge: "NeoForge" } as Record<string, string>)[inst.loader] ?? "Vanilla";
      presence = {
        details: hide || !d.showBuild ? (ru ? "Играет в Minecraft" : "Playing Minecraft") : `${ru ? "Сборка" : "Build"}: ${inst.name}`.slice(0, 120),
        state: `${loader} ${inst.mcVersion}${where}`,
        start: d.showTime ? gameStarts.current[id] ?? Math.floor(Date.now() / 1000) : null,
        largeText: "AniLauncher",
      };
    } else if (d.showIdle) {
      presence = {
        details: ru ? "В лаунчере" : "In the launcher",
        state: ru ? "Выбирает, во что поиграть" : "Picking what to play",
        start: d.showTime ? launcherStart.current : null,
        largeText: "AniLauncher",
      };
    }
    void api.discordPresence(presence).catch(() => {});
  }, [loaded, running, instances, linked, settings.discord, settings.streamer, settings.lang]);

  // A quiet check for a new launcher version shortly after start
  useEffect(() => {
    if (!loaded || !settingsRef.current.ui.autoUpdateCheck) return;
    const timer = window.setTimeout(async () => {
      try {
        if (!(await api.updatesConfigured())) return;
        const u = await api.checkUpdate();
        if (!u) return;
        setUpdate(u);
        const ru = settingsRef.current.lang !== "en";
        toast(ru ? `Вышла новая версия AniLauncher ${u.version}` : `AniLauncher ${u.version} is out`, "ok", { label: ru ? "Подробнее" : "Details", run: () => openUpdate(true) }, false);
      } catch {
        /* offline or GitHub unavailable: try next time */
      }
    }, 6000);
    return () => window.clearTimeout(timer);
  }, [loaded, toast]);

  // The tray menu follows builds, the running game and YouTuber mode
  useEffect(() => {
    const byRecent = [...instances].sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0));
    const name = (id: string) => instances.find((i) => i.id === id)?.name ?? id;
    void api
      .trayUpdate({
        builds: byRecent.slice(0, 5).map((i) => ({ id: i.id, name: i.name })),
        running: running.map((id) => ({ id, name: name(id) })),
        linked: Object.keys(linked),
        streamer: settings.streamer,
        lang: settings.lang,
      })
      .catch(() => {});
  }, [instances, running, linked, settings.streamer, settings.lang]);

  // Clicks in the tray menu
  useEffect(() => {
    const off = listen<{ action: string; id: string }>("tray", (e) => {
      const { action, id } = e.payload;
      if (action === "play") void play(id);
      else if (action === "stop") void api.stopInstance(id).catch((err) => toast(errText(err), "error"));
      else if (action === "console") {
        void getCurrentWindow().show().then(() => getCurrentWindow().setFocus()).catch(() => {});
        setConsoleFor(id);
      } else if (action === "remote") setRemoteFor(id);
      else if (action === "streamer") {
        const s = settingsRef.current;
        updateSettings({ streamer: !s.streamer });
      }
    });
    return () => void off.then((f) => f());
  }, [play, toast, updateSettings]);

  const go = useCallback((p: Page) => {
    sfx.click();
    setModsSearch(null);
    setPage(p);
    const nav: NavPage[] = ["home", "builds", "mods", "skins", "shots", "news"];
    const ui = settingsRef.current.ui;
    if (ui.startPage === "last" && nav.includes(p as NavPage) && ui.lastPage !== p) updateSettings({ ui: { ...ui, lastPage: p as NavPage } });
  }, [updateSettings]);

  const value: Store = {
    update,
    setUpdate,
    updateOpen,
    openUpdate,
    remoteFor,
    openRemote: setRemoteFor,
    notify,
    linked,
    page, go, settings, updateSettings, t, accounts, setAccounts, instances, reloadInstances,
    selected, select, running, preparing, play, feed, feedError, toast, toasts,
    logs, setLog, crashes, consoleFor, openConsole: setConsoleFor, check, setCheck,
    season: seasonPreview ?? (!settings.streamer ? activeSeason(settings) : null),
    mask: (text: string) => {
      if (!settings.streamer) return text;
      let out = text
        .replace(/\b\d{1,3}(\.\d{1,3}){3}(:\d+)?\b/g, "███.███")
        .replace(/Connecting to [^\s,]+/g, "Connecting to ███");
      for (const a of accounts.accounts) if (a.name.length >= 3) out = out.split(a.name).join("Player");
      if (settings.profile.nick.length >= 3) out = out.split(settings.profile.nick).join("Player");
      return out;
    },
    previewSeason,
    modsSearch,
    searchMods: (query: string, instanceId?: string) => {
      if (instanceId) updateSettings({ selectedInstance: instanceId });
      setModsSearch(query);
      setPage("mods");
    },
  };

  if (!loaded) return null;
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

/** Today's celebration, if the player has it switched on. */
function activeSeason(s: Settings): SeasonEvent {
  const ev = seasonToday(s.events.birthday ? s.birthday : null);
  if (ev && !s.events[ev]) return null;
  return ev;
}

function hexRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  const n = parseInt(h.length === 3 ? h.split("").map((c) => c + c).join("") : h.slice(0, 6), 16);
  return [n >> 16, (n >> 8) & 255, n & 255];
}
function lighten(hex: string, k: number) {
  const [r, g, b] = hexRgb(hex).map((v) => Math.round(v + (255 - v) * k));
  return `#${((1 << 24) | (r << 16) | (g << 8) | b).toString(16).slice(1)}`;
}
function luminance(hex: string) {
  const [r, g, b] = hexRgb(hex);
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255;
}
