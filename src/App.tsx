import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore, type Page } from "./store";
import { Icon } from "./components/Icon";
import { Avatar } from "./components/Bits";
import { sfx } from "./sounds";
import { Home } from "./pages/Home";
import { Builds } from "./pages/Builds";
import { Profile } from "./pages/Profile";
import { SettingsPage } from "./pages/Settings";
import { News } from "./pages/News";
import { Mods } from "./pages/Mods";
import { Skins } from "./pages/Skins";
import { Screenshots } from "./pages/Screenshots";
import { Console } from "./pages/Console";
import { ConflictDialog } from "./pages/ConflictDialog";
import { SeasonalLayer } from "./components/Seasonal";
import { DropZone } from "./pages/Extras";
import type { Key } from "./i18n";
import { api, type NavPage } from "./api";
import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { GameRemote } from "./pages/GameRemote";
import { UpdateDialog } from "./pages/Care";

function TitleBar() {
  const { t, settings, updateSettings, toast } = useStore();
  const win = getCurrentWindow();
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar__brand" data-tauri-drag-region>
        <img src="/logo.png" width={22} height={22} alt="" />
        <span>AniLauncher</span>
      </div>
      <div className="titlebar__btns">
        <button
          className={`winbtn winbtn--yt ${settings.streamer ? "is-on" : ""}`}
          title={t("yt.mode")}
          aria-label={t("yt.mode")}
          aria-pressed={settings.streamer}
          onClick={() => {
            const on = !settings.streamer;
            updateSettings({ streamer: on });
            toast(on ? t("yt.on") : t("yt.off"), on ? "ok" : "info");
          }}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
            <rect x="2.5" y="5" width="19" height="14" rx="4" fill="none" stroke="currentColor" strokeWidth="1.8" />
            <path d="M10 9.2v5.6l4.8-2.8z" fill="currentColor" />
          </svg>
        </button>
        <button className="winbtn" aria-label={t("win.min")} onClick={() => void win.minimize()}>
          <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M1 6h10" stroke="currentColor" strokeWidth="1.4" /></svg>
        </button>
        <button className="winbtn" aria-label={t("win.max")} onClick={() => void win.toggleMaximize()}>
          <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><rect x="1.5" y="1.5" width="9" height="9" fill="none" stroke="currentColor" strokeWidth="1.4" /></svg>
        </button>
        <button
          className="winbtn winbtn--close"
          aria-label={t("win.close")}
          title={settings.ui.closeToTray ? (settings.lang === "ru" ? "Свернуть в трей" : "Hide to tray") : undefined}
          onClick={() => void (settings.ui.closeToTray ? api.hideToTray() : win.close())}
        >
          <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M1.5 1.5l9 9M10.5 1.5l-9 9" stroke="currentColor" strokeWidth="1.4" /></svg>
        </button>
      </div>
    </header>
  );
}

const mainNav: Array<{ page: Page; icon: string; key: Key }> = [
  { page: "home", icon: "home", key: "nav.home" },
  { page: "builds", icon: "builds", key: "nav.builds" },
  { page: "mods", icon: "mods", key: "nav.mods" },
  { page: "skins", icon: "skins", key: "nav.skins" },
  { page: "shots", icon: "shots", key: "nav.shots" },
];
const bottomNav: Array<{ page: Page; icon: string; key: Key }> = [
  { page: "news", icon: "news", key: "nav.news" },
  { page: "settings", icon: "settings", key: "nav.settings" },
];

function Sidebar() {
  const { page, go, t, settings, accounts, linked, update, openUpdate } = useStore();
  const [appVersion, setAppVersion] = useState("1.3.0");
  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => {});
  }, []);
  const playing = Object.values(linked).find((w) => w && w.state === "enter");
  const active = accounts.accounts.find((a) => a.active);
  const nick = settings.streamer ? "Player" : settings.profile.nick || active?.name || "Anidius";
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const sub = playing
    ? playing.kind === "singleplayer"
      ? L(`Играет · ${playing.name}`, `Playing · ${playing.name}`)
      : playing.kind === "multiplayer" && !settings.streamer
        ? L(`На сервере · ${playing.name}`, `On ${playing.name}`)
        : L("Играет", "Playing")
    : settings.streamer
    ? t("yt.mode")
    : active
      ? `${active.kind === "msa" ? t("acc.licensed") : t("acc.offline")} · ${active.name}`
      : t("home.signIn");
  const item = (n: { page: Page; icon: string; key: Key }) => (
    <button
      key={n.page}
      className={`nav ${page === n.page ? "nav--active" : ""}`}
      onClick={() => go(n.page)}
      onMouseEnter={sfx.hover}
      aria-current={page === n.page ? "page" : undefined}
      title={settings.ui.sidebar === "compact" ? t(n.key) : undefined}
      aria-label={t(n.key)}
    >
      <Icon name={n.icon} />
      <span className="nav__label">{t(n.key)}</span>
    </button>
  );
  return (
    <aside className="sidebar">
      <button className={`profile-chip ${page === "profile" ? "profile-chip--active" : ""}`} onClick={() => go("profile")}>
        <Avatar path={settings.profile.avatar} name={nick} frame={settings.profile.frame} />
        <span className="profile-chip__text">
          <span className="profile-chip__nick">{nick}</span>
          <span className={`profile-chip__sub ${playing ? "profile-chip__sub--playing" : ""}`}>{sub}</span>
        </span>
      </button>
      <nav className="nav-list" aria-label="Разделы">{mainNav.filter((n) => n.page === "home" || !settings.ui.hiddenNav.includes(n.page as NavPage)).map(item)}</nav>
      <div className="grow" />
      <nav className="nav-list nav-list--bottom" aria-label="Прочее">
        {bottomNav.filter((n) => !settings.ui.hiddenNav.includes(n.page as NavPage)).map(item)}
        <span className="version">
          AniLauncher {appVersion}
          {update && (
            <button className="version__update" onClick={() => openUpdate(true)}>
              {settings.lang === "ru" ? `→ ${update.version}` : `→ ${update.version}`}
            </button>
          )}
        </span>
      </nav>
    </aside>
  );
}

function Toasts() {
  const { toasts, settings } = useStore();
  return (
    <div className={`toasts toasts--${settings.ui.toastPos}`} aria-live="polite">
      {toasts.map((x) => (
        <div key={x.id} className={`toast toast--${x.kind}`}>
          <span>{x.text}</span>
          {x.action && (
            <button className="btn btn--small toast__action" onClick={x.action.run}>
              {x.action.label}
            </button>
          )}
        </div>
      ))}
    </div>
  );
}

export function App() {
  const { page, consoleFor, openConsole, season, settings, remoteFor, openRemote, instances, updateOpen, openUpdate } = useStore();
  const remoteInst = instances.find((i) => i.id === remoteFor);
  // Alt+F4 and the taskbar's "Close window" follow the same setting as the ✕ button
  const closeToTray = settings.ui.closeToTray;
  useEffect(() => {
    const off = getCurrentWindow().onCloseRequested((e) => {
      if (closeToTray) {
        e.preventDefault();
        void api.hideToTray();
      }
    });
    return () => void off.then((f) => f());
  }, [closeToTray]);
  return (
    <div className="app">
      <TitleBar />
      <div className="body">
        <Sidebar />
        <main className="content" key={page}>
          {page === "home" && <Home />}
          {page === "builds" && <Builds />}
          {page === "profile" && <Profile />}
          {page === "settings" && <SettingsPage />}
          {page === "news" && <News />}
          {page === "mods" && <Mods />}
          {page === "skins" && <Skins />}
          {page === "shots" && <Screenshots />}
        </main>
      </div>
      {consoleFor && <Console instanceId={consoleFor} onClose={() => openConsole(null)} />}
      {remoteInst && <GameRemote inst={remoteInst} onClose={() => openRemote(null)} />}
      {updateOpen && <UpdateDialog onClose={() => openUpdate(false)} />}
      <ConflictDialog />
      {(settings.events.everywhere || page === "home") && settings.events.particles > 0 && (
        <SeasonalLayer event={season} animate={settings.animations} density={settings.events.particles} />
      )}
      <DropZone />
      <Toasts />
    </div>
  );
}
