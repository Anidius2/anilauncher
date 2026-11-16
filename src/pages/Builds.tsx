import { useEffect, useRef, useState } from "react";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { WorldsDialog } from "./Worlds";
import { GameRemote } from "./GameRemote";
import { ReportDialog } from "./Care";
import { Modpacks } from "./Modpacks";
import { SnapshotsDialog } from "./Snapshots";
import { ControlsDialog, FriendDialog } from "./Extras";
import { api, artSrc, errText, type Instance } from "../api";
import { useStore } from "../store";
import { DefaultArt, formatPlaytime, loaderLabel } from "../components/Bits";
import { Icon } from "../components/Icon";
import { BuildDialog } from "./BuildDialog";
import { mojangArt } from "./Home";
import { sfx } from "../sounds";
import type { Key } from "../i18n";

const folders = ["mods", "saves", "resourcepacks", "shaderpacks", "screenshots", "config", "logs"] as const;

function BuildMenu({ inst, onWorlds, onSnapshots, onControls, onRemote, onReport, onClose }: { inst: Instance; onWorlds: () => void; onSnapshots: () => void; onControls: () => void; onRemote: () => void; onReport: () => void; onClose: () => void }) {
  const { t, toast, reloadInstances, openConsole, running, linked, settings } = useStore();
  const ref = useRef<HTMLDivElement>(null);
  const [foldersOpen, setFoldersOpen] = useState(false);
  useEffect(() => {
    const close = (e: MouseEvent) => ref.current && !ref.current.contains(e.target as Node) && onClose();
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [onClose]);
  const run = (f: () => Promise<unknown> | void) => async () => {
    onClose();
    try {
      await f();
    } catch (e) {
      toast(errText(e), "error");
    }
  };
  return (
    <div className="menu" ref={ref} role="menu">
      {inst.id in linked && (
        <button role="menuitem" onClick={run(onRemote)}>
          <Icon name="sparkle" size={16} /> {settings.lang === "ru" ? "Пульт игры" : "Game remote"}
        </button>
      )}
      <button role="menuitem" onClick={run(() => openConsole(inst.id))}>
        <Icon name="terminal" size={16} /> {t("builds.console")}
      </button>
      <button role="menuitem" onClick={run(onReport)}>
        <Icon name="download" size={16} /> {settings.lang === "ru" ? "Отчёт об ошибке" : "Bug report"}
      </button>
      <button role="menuitem" onClick={run(onWorlds)}>
        <Icon name="globe" size={16} /> {t("builds.worlds")}
      </button>
      <button role="menuitem" onClick={run(onSnapshots)}>
        <Icon name="undo" size={16} /> {t("snap.title")}
      </button>
      <button role="menuitem" onClick={run(onControls)}>
        <Icon name="grid" size={16} /> {t("builds.controls")}
      </button>
      <button
        role="menuitem"
        onClick={run(async () => {
          await api.duplicateInstance(inst.id);
          await reloadInstances();
          toast(t("builds.duplicated"), "ok");
        })}
      >
        <Icon name="copy" size={16} /> {t("builds.duplicate")}
      </button>
      <button
        role="menuitem"
        onClick={run(async () => {
          const dest = await save({ defaultPath: `${inst.name}.mrpack`, filters: [{ name: "Modrinth pack", extensions: ["mrpack"] }] });
          if (!dest) return;
          await api.exportPack(inst.id, dest);
          toast(t("builds.exported"), "ok");
        })}
      >
        <Icon name="upload" size={16} /> {t("builds.export")}
      </button>
      <button role="menuitem" aria-expanded={foldersOpen} onClick={() => setFoldersOpen((o) => !o)}>
        <Icon name="folder" size={16} /> {t("builds.folders")} <span className="grow" /> <Icon name="chevron" size={14} />
      </button>
      {foldersOpen && (
        <div className="menu__sub">
          <button role="menuitem" onClick={run(() => api.openInstanceFolder(inst.id))}>
            {t("builds.folder")}
          </button>
          {folders.map((f) => (
            <button key={f} role="menuitem" onClick={run(() => api.openInstanceFolder(inst.id, f))}>
              {f === "config" ? "config" : f === "logs" ? "logs" : t(`builds.open.${f}` as Key)}
            </button>
          ))}
        </div>
      )}
      <span className="menu__sep" />
      <button
        role="menuitem"
        className="menu__danger"
        disabled={running.includes(inst.id)}
        onClick={run(async () => {
          if (!(await confirm(t("builds.deleteConfirm", { name: inst.name }), { title: "AniLauncher", kind: "warning" }))) return;
          await api.deleteInstance(inst.id);
          await reloadInstances();
        })}
      >
        <Icon name="trash" size={16} /> {t("builds.delete")}
      </button>
    </div>
  );
}

function BuildTile({ inst, onEdit, onWorlds, onSnapshots, onControls, onRemote, onReport }: { inst: Instance; onEdit: () => void; onWorlds: () => void; onSnapshots: () => void; onControls: () => void; onRemote: () => void; onReport: () => void }) {
  const { t, settings, play, running, preparing, select, selected, reloadInstances, toast, go, feed } = useStore();
  const [menu, setMenu] = useState(false);
  const src = artSrc(inst.art) ?? mojangArt(feed, inst.id);
  const isRunning = running.includes(inst.id);
  const prog = preparing[inst.id];

  return (
    <div className={`card build-tile ${selected?.id === inst.id ? "card--active" : ""} ${menu ? "build-tile--menu" : ""}`}>
      <button
        className="build-tile__art"
        onClick={() => {
          sfx.click();
          select(inst.id);
          go("home");
        }}
        aria-label={inst.name}
      >
        {src ? <img className="art-fill" src={src} alt="" loading="lazy" /> : <DefaultArt />}
        <span className="build-tile__badge">{loaderLabel(inst.loader)}</span>
        {isRunning && <span className="build-tile__live">● LIVE</span>}
      </button>
      <button
        className={`build-tile__pin ${inst.pinned ? "is-on" : ""}`}
        title={inst.pinned ? t("builds.unpin") : t("builds.pin")}
        aria-label={inst.pinned ? t("builds.unpin") : t("builds.pin")}
        aria-pressed={!!inst.pinned}
        onClick={async () => {
          try {
            await api.updateInstance({ ...inst, pinned: !inst.pinned });
            await reloadInstances();
          } catch (e) {
            toast(errText(e), "error");
          }
        }}
      >
        <Icon name="star" size={16} />
      </button>
      <div className="build-tile__body">
        <div className="build-tile__info">
          <span className="build-tile__name ellipsis">{inst.name}</span>
          <span className="muted small">
            {inst.mcVersion}
            {inst.loaderVersion ? ` · ${inst.loaderVersion}` : ""} ·{" "}
            {inst.playSeconds > 0 ? `${t("builds.played")}: ${formatPlaytime(inst.playSeconds, settings.lang)}` : t("builds.never")}
          </span>
        </div>
        <div className="build-tile__actions">
          <button className="btn btn--primary btn--small" disabled={isRunning || !!prog} onClick={() => void play(inst.id)}>
            <Icon name="play" size={14} /> {prog ? `${Math.round((prog.done / Math.max(1, prog.total)) * 100)}%` : t("home.play")}
          </button>
          <button className="icon-btn" title={t("builds.edit")} aria-label={t("builds.edit")} onClick={onEdit}>
            <Icon name="edit" size={18} />
          </button>
          <div className="menu-wrap">
            <button className="icon-btn" title={t("builds.more")} aria-label={t("builds.more")} aria-expanded={menu} onClick={() => setMenu((m) => !m)}>
              <Icon name="more" size={18} />
            </button>
            {menu && <BuildMenu inst={inst} onWorlds={onWorlds} onSnapshots={onSnapshots} onControls={onControls} onRemote={onRemote} onReport={onReport} onClose={() => setMenu(false)} />}
          </div>
        </div>
      </div>
    </div>
  );
}

export function Builds() {
  const { t, instances, toast, reloadInstances, select } = useStore();
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Instance | null>(null);
  const [creating, setCreating] = useState(false);
  const [worlds, setWorlds] = useState<Instance | null>(null);
  const [catalog, setCatalog] = useState(false);
  const [snaps, setSnaps] = useState<Instance | null>(null);
  const [controls, setControls] = useState<Instance | null>(null);
  const [remote, setRemote] = useState<Instance | null>(null);
  const [report, setReport] = useState<Instance | null>(null);
  const [friend, setFriend] = useState(false);
  const [importing, setImporting] = useState(false);

  const doImport = async () => {
    try {
      const file = await open({ multiple: false, filters: [{ name: "Modrinth / Prism", extensions: ["mrpack", "zip"] }] });
      if (typeof file !== "string") return;
      setImporting(true);
      const inst = await api.importPack(file);
      await reloadInstances();
      select(inst.id);
      toast(t("builds.imported", { name: inst.name }), "ok");
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setImporting(false);
    }
  };

  const list = instances.filter((i) => i.name.toLowerCase().includes(query.trim().toLowerCase()));
  return (
    <div className="page">
      <div className="page-head">
        <h1>{t("builds.title")}</h1>
        <div className="page-head__tools">
          <input className="search" placeholder={t("builds.search")} value={query} onChange={(e) => setQuery(e.target.value)} aria-label={t("builds.search")} />
          <button className="btn" onClick={() => setFriend(true)} title={t("builds.wayFriend")}>
            <Icon name="link" size={18} /> {t("builds.friend")}
          </button>
          <button className="btn" onClick={() => setCatalog(true)} title={t("builds.wayPacks")}>
            <Icon name="rocket" size={18} /> {t("builds.modpacks")}
          </button>
          <button className="btn" onClick={doImport} disabled={importing} title={t("builds.wayImport")}>
            <Icon name="download" size={18} /> {importing ? t("builds.importing") : t("builds.import")}
          </button>
          <button className="btn btn--primary" onClick={() => setCreating(true)}>
            <Icon name="plus" size={18} /> {t("home.newBuild")}
          </button>
        </div>
      </div>
      {instances.length === 0 && (
        <div className="ways">
          <p className="ways__lead">{t("builds.emptyLead")}</p>
          {[
            { icon: "plus", title: t("home.newBuild"), text: t("builds.wayNew"), run: () => setCreating(true), main: true },
            { icon: "rocket", title: t("builds.modpacks"), text: t("builds.wayPacks"), run: () => setCatalog(true) },
            { icon: "link", title: t("builds.friend"), text: t("builds.wayFriend"), run: () => setFriend(true) },
            { icon: "download", title: t("builds.import"), text: t("builds.wayImport"), run: doImport },
          ].map((w) => (
            <button key={w.title} className={`way ${w.main ? "way--main" : ""}`} onClick={w.run} onMouseEnter={sfx.hover}>
              <span className="way__icon">
                <Icon name={w.icon as "plus"} size={24} />
              </span>
              <strong>{w.title}</strong>
              <span className="muted small">{w.text}</span>
            </button>
          ))}
        </div>
      )}
      <div className="grid grid--builds">
        {list.map((i) => (
          <BuildTile key={i.id} inst={i} onEdit={() => setEditing(i)} onWorlds={() => setWorlds(i)} onSnapshots={() => setSnaps(i)} onControls={() => setControls(i)} onRemote={() => setRemote(i)} onReport={() => setReport(i)} />
        ))}
      </div>
      {creating && <BuildDialog onClose={() => setCreating(false)} />}
      {editing && <BuildDialog instance={editing} onClose={() => setEditing(null)} />}
      {worlds && <WorldsDialog inst={worlds} onClose={() => setWorlds(null)} />}
      {catalog && <Modpacks onClose={() => setCatalog(false)} />}
      {snaps && <SnapshotsDialog inst={snaps} onClose={() => setSnaps(null)} />}
      {controls && <ControlsDialog inst={controls} onClose={() => setControls(null)} />}
      {remote && <GameRemote inst={remote} onClose={() => setRemote(null)} />}
      {report && <ReportDialog instanceId={report.id} onClose={() => setReport(null)} />}
      {friend && <FriendDialog onClose={() => setFriend(false)} />}
    </div>
  );
}
