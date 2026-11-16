// Quick start, controls profiles, "like a friend's", dropped files, base mods.
import { useCallback, useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, errText, fileSrc, type ControlProfile, type DropInfo, type Instance, type Loader, type QuickItem, type SharedFound } from "../api";
import { useStore } from "../store";
import { Modal, loaderLabel } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";
import type { Key } from "../i18n";

// ---------- quick start ----------

export function QuickStart() {
  const { t, play, running, preparing, settings, mask } = useStore();
  const [items, setItems] = useState<QuickItem[] | null>(null);
  useEffect(() => {
    api.quickStart().then(setItems).catch(() => setItems([]));
  }, [running.length]);
  if (settings.widgets.quick === false || !items || items.length === 0) return null;
  return (
    <section>
      <div className="section-head">
        <h2>{t("quick.title")}</h2>
      </div>
      <div className="quick">
        {items.map((q) => {
          const busy = running.includes(q.instanceId) || !!preparing[q.instanceId];
          return (
            <button
              key={`${q.kind}-${q.instanceId}-${q.target}`}
              className="card quick__item"
              disabled={busy}
              onMouseEnter={sfx.hover}
              onClick={() => void play(q.instanceId, false, { kind: q.kind, target: q.target })}
            >
              <span className={`quick__icon quick__icon--${q.kind}`}>
                {q.icon ? <img src={fileSrc(q.icon) ?? ""} alt="" /> : <Icon name={q.kind === "world" ? "globe" : "link"} size={22} />}
              </span>
              <span className="quick__text">
                <span className="muted small">{q.kind === "world" ? t("quick.world") : t("quick.server")}</span>
                <strong className="ellipsis">{q.kind === "server" && settings.streamer ? t("yt.hidden") : mask(q.target)}</strong>
                <span className="muted small ellipsis">{q.instanceName}</span>
              </span>
              <span className="quick__play">
                <Icon name="play" size={14} />
              </span>
            </button>
          );
        })}
      </div>
    </section>
  );
}

// ---------- controls profiles ----------

export function ControlsDialog({ inst, onClose }: { inst: Instance; onClose: () => void }) {
  const { t, toast, linked } = useStore();
  const [list, setList] = useState<ControlProfile[]>([]);
  const [name, setName] = useState(inst.name);
  useEffect(() => {
    api.listControls().then(setList).catch(() => setList([]));
  }, []);
  return (
    <Modal title={`${t("ctl.title")} · ${inst.name}`} onClose={onClose} wide>
      <p className="muted small">{t("ctl.hint")}</p>
      <div className="inline-form">
        <input value={name} maxLength={40} placeholder={t("ctl.namePh")} onChange={(e) => setName(e.target.value)} aria-label={t("ctl.namePh")} />
        <button
          className="btn btn--primary"
          onClick={async () => {
            try {
              setList(await api.saveControls(inst.id, name));
              sfx.success();
            } catch (e) {
              toast(errText(e), "error");
            }
          }}
        >
          {t("ctl.save")}
        </button>
      </div>
      {list.length === 0 && <p className="empty">{t("ctl.empty")}</p>}
      <div className="content-list">
        {list.map((p) => (
          <div key={p.name} className="content-row">
            <span className="mod-card__icon mod-card__icon--small">
              <Icon name="grid" size={20} />
            </span>
            <span className="content-row__text">
              <strong className="ellipsis">{p.name}</strong>
              <span className="muted small">{t("ctl.keys", { n: Object.keys(p.keys).length, from: p.from })}</span>
            </span>
            <button
              className="btn btn--small btn--primary"
              onClick={async () => {
                try {
                  if (inst.id in linked) {
                    // the game is running with the link: change keys live instead of refusing
                    await api.linkSend(inst.id, { cmd: "setKeys", keys: p.keys });
                    sfx.success();
                    toast(t("ctl.applied", { n: Object.keys(p.keys).length }), "ok");
                    return;
                  }
                  const n = await api.applyControls(inst.id, p.name);
                  sfx.success();
                  toast(t("ctl.applied", { n }), "ok");
                } catch (e) {
                  toast(errText(e), "error");
                }
              }}
            >
              {t("ctl.apply")}
            </button>
            <button
              className="icon-btn icon-btn--danger"
              aria-label={t("worlds.delete")}
              onClick={async () => setList(await api.deleteControls(p.name).catch(() => list))}
            >
              <Icon name="trash" size={16} />
            </button>
          </div>
        ))}
      </div>
    </Modal>
  );
}

// ---------- base mods ----------

/** Installs the base mods set for a loader into a build. Returns how many were added. */
export async function installBaseMods(instanceId: string, loader: string, base: Record<string, Array<{ projectId: string }>>): Promise<number> {
  const list = base[loader] ?? [];
  let n = 0;
  for (const m of list) {
    try {
      await api.installContent(instanceId, "mod", m.projectId, null);
      n++;
    } catch {
      /* no version for this game version: skip */
    }
  }
  return n;
}

const modLoaders: Loader[] = ["fabric", "quilt", "forge", "neoforge"];

export function BaseModsSettings() {
  const { t, settings, updateSettings, toast, instances } = useStore();
  const [loader, setLoader] = useState<Loader>("fabric");
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<Array<{ projectId: string; title: string; iconUrl?: string | null }>>([]);
  const [busy, setBusy] = useState(false);
  const list = settings.baseMods[loader] ?? [];

  useEffect(() => {
    if (!query.trim()) {
      setHits([]);
      return;
    }
    const id = setTimeout(() => {
      api
        .search(query, "mod", null, loader, 0, "relevance")
        .then((r) => setHits(r.hits.slice(0, 6)))
        .catch(() => setHits([]));
    }, 300);
    return () => clearTimeout(id);
  }, [query, loader]);

  const setList = (next: typeof list) => updateSettings({ baseMods: { ...settings.baseMods, [loader]: next } });

  return (
    <section className="panel">
      <h2>{t("base.title")}</h2>
      <p className="muted small">{t("base.hint")}</p>
      <div className="seg seg--panel">
        {modLoaders.map((l) => (
          <button key={l} className={`seg__btn ${loader === l ? "seg__btn--on" : ""}`} onClick={() => setLoader(l)}>
            {loaderLabel(l)}
            {(settings.baseMods[l]?.length ?? 0) > 0 ? ` · ${settings.baseMods[l].length}` : ""}
          </button>
        ))}
      </div>
      {list.length === 0 && <p className="muted small">{t("base.empty")}</p>}
      <div className="chips-list">
        {list.map((m) => (
          <span key={m.projectId} className="mod-chip">
            {m.icon ? <img src={m.icon} alt="" /> : <Icon name="mods" size={14} />}
            {m.title}
            <button aria-label={t("mods.remove")} onClick={() => setList(list.filter((x) => x.projectId !== m.projectId))}>
              <Icon name="close" size={12} />
            </button>
          </span>
        ))}
      </div>
      <div className="base-search">
        <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("base.search")} aria-label={t("base.search")} />
        {hits.length > 0 && (
          <div className="base-search__hits">
            {hits.map((h) => (
              <button
                key={h.projectId}
                disabled={list.some((x) => x.projectId === h.projectId)}
                onClick={() => {
                  setList([...list, { projectId: h.projectId, title: h.title, icon: h.iconUrl ?? null }]);
                  setQuery("");
                  sfx.click();
                }}
              >
                {h.iconUrl ? <img src={h.iconUrl} alt="" /> : <Icon name="mods" size={16} />} {h.title}
              </button>
            ))}
          </div>
        )}
      </div>
      {list.length > 0 && (
        <button
          className="btn"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            toast(t("base.installing"));
            let total = 0;
            for (const i of instances.filter((x) => x.loader === loader)) total += await installBaseMods(i.id, loader, settings.baseMods);
            setBusy(false);
            toast(t("base.done", { n: total }), "ok");
          }}
        >
          {t("base.applyAll", { loader: loaderLabel(loader) })}
        </button>
      )}
    </section>
  );
}

// ---------- like a friend's ----------

export function FriendDialog({ onClose }: { onClose: () => void }) {
  const { t, toast, instances, selected, reloadInstances, select, settings } = useStore();
  const [text, setText] = useState("");
  const [target, setTarget] = useState<string>(selected?.id ?? "new");
  const [name, setName] = useState(settings.lang === "ru" ? "Как у друга" : "Friend's build");
  const [loader, setLoader] = useState<Loader>("fabric");
  const [mc, setMc] = useState("");
  const [versions, setVersions] = useState<string[]>([]);
  const [found, setFound] = useState<SharedFound[] | null>(null);
  const [notFound, setNotFound] = useState<string[]>([]);
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    api
      .listGameVersions(loader)
      .then((v) => {
        const rel = v.filter((x) => x.kind === "release").map((x) => x.id);
        setVersions(rel);
        setMc((cur) => (cur && rel.includes(cur) ? cur : rel[0] ?? ""));
      })
      .catch(() => {});
  }, [loader]);

  const inst = instances.find((i) => i.id === target);
  const useLoader = inst ? inst.loader : loader;
  const useMc = inst ? inst.mcVersion : mc;

  const find = async () => {
    setBusy("find");
    try {
      const r = await api.resolveShare(text, useLoader, useMc);
      setFound(r.items);
      setNotFound(r.notFound);
      setChosen(new Set(r.items.filter((x) => x.projectType !== "modpack" && x.available !== false).map((x) => x.projectId)));
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(null);
    }
  };

  const install = async () => {
    const items = (found ?? []).filter((f) => chosen.has(f.projectId) && f.projectType !== "modpack");
    if (!items.length) return;
    try {
      let id = target;
      if (target === "new") {
        const created = await api.createInstance({ name, mcVersion: mc, loader, loaderVersion: null, art: null });
        id = created.id;
        await installBaseMods(id, loader, settings.baseMods);
      }
      const failed: string[] = [];
      for (let i = 0; i < items.length; i++) {
        setBusy(t("friend.installing", { i: i + 1, n: items.length }));
        const it = items[i];
        const kind = it.projectType === "resourcepack" ? "resourcepack" : it.projectType === "shader" ? "shader" : "mod";
        try {
          await api.installContent(id, kind, it.projectId, null);
        } catch {
          failed.push(it.title);
        }
      }
      await reloadInstances();
      select(id);
      sfx.success();
      toast(t("friend.done", { n: items.length - failed.length }), "ok");
      if (failed.length) toast(t("friend.failed", { list: failed.join(", ") }), "error");
      onClose();
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(null);
    }
  };

  const modpack = found?.find((f) => f.projectType === "modpack");

  return (
    <Modal title={t("friend.title")} onClose={onClose} wide xl>
      <p className="muted small">{t("friend.hint")}</p>
      <textarea className="friend__text" value={text} onChange={(e) => setText(e.target.value)} placeholder={t("friend.ph")} rows={6} />
      <div className="field-row">
        <label className="field">
          <span>{t("friend.target")}</span>
          <select value={target} onChange={(e) => setTarget(e.target.value)}>
            <option value="new">+ {t("friend.newBuild")}</option>
            {instances.map((i) => (
              <option key={i.id} value={i.id}>
                {i.name} · {loaderLabel(i.loader)} {i.mcVersion}
              </option>
            ))}
          </select>
        </label>
        {target === "new" && (
          <>
            <label className="field">
              <span>{t("create.name")}</span>
              <input value={name} maxLength={48} onChange={(e) => setName(e.target.value)} />
            </label>
            <label className="field">
              <span>{t("create.loader")}</span>
              <select value={loader} onChange={(e) => setLoader(e.target.value as Loader)}>
                {(["fabric", "quilt", "neoforge", "forge"] as Loader[]).map((l) => (
                  <option key={l} value={l}>
                    {loaderLabel(l)}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>{t("create.version")}</span>
              <select value={mc} onChange={(e) => setMc(e.target.value)}>
                {versions.slice(0, 80).map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </select>
            </label>
          </>
        )}
      </div>
      <div className="toolbar">
        <button className="btn btn--primary" disabled={!text.trim() || busy !== null} onClick={find}>
          <Icon name="link" size={16} /> {busy === "find" ? t("mods.loading") : t("friend.find")}
        </button>
        {found && <span className="muted small">{t("friend.found", { n: found.length })}</span>}
      </div>
      {modpack && (
        <button
          className="notice notice--action"
          onClick={async () => {
            setBusy("pack");
            try {
              const i = await api.installModpack(modpack.projectId);
              await reloadInstances();
              select(i.id);
              toast(t("mp.installed", { name: i.name }), "ok");
              onClose();
            } catch (e) {
              toast(errText(e), "error");
            } finally {
              setBusy(null);
            }
          }}
        >
          <Icon name="rocket" size={18} /> {t("friend.modpack")}: <strong>{modpack.title}</strong>
        </button>
      )}
      {found && found.length > 0 && (
        <div className="friend__list">
          {found
            .filter((f) => f.projectType !== "modpack")
            .map((f) => (
              <label key={f.projectId} className={`friend__item ${chosen.has(f.projectId) ? "is-on" : ""} ${f.available === false ? "is-missing" : ""}`}>
                <input
                  type="checkbox"
                  disabled={f.available === false}
                  checked={chosen.has(f.projectId)}
                  onChange={(e) => {
                    const n = new Set(chosen);
                    if (e.target.checked) n.add(f.projectId);
                    else n.delete(f.projectId);
                    setChosen(n);
                  }}
                />
                {f.icon ? <img src={f.icon} alt="" /> : <Icon name="mods" size={18} />}
                <span className="content-row__text">
                  <strong className="ellipsis">{f.title}</strong>
                  <span className={`small ellipsis ${f.available === false ? "friend__missing" : "muted"}`}>
                    {f.available === false ? t("friend.missing", { mc: useMc, loader: loaderLabel(useLoader) }) : f.source}
                  </span>
                </span>
              </label>
            ))}
        </div>
      )}
      {notFound.length > 0 && <p className="muted small">{t("friend.notFound", { list: notFound.slice(0, 10).join(", ") })}</p>}
      <div className="modal__foot">
        <button className="btn" onClick={onClose}>
          {t("create.cancel")}
        </button>
        <button className="btn btn--primary" disabled={!chosen.size || busy !== null} onClick={install}>
          {busy && busy !== "find" && busy !== "pack" ? busy : t("friend.install", { n: chosen.size })}
        </button>
      </div>
    </Modal>
  );
}

// ---------- drop files anywhere ----------

export function DropZone() {
  const { t, toast, instances, reloadInstances, select } = useStore();
  const [over, setOver] = useState(false);
  const [queue, setQueue] = useState<DropInfo[]>([]);
  const [target, setTarget] = useState("");
  const [showAll, setShowAll] = useState(false);

  useEffect(() => {
    let off: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent(async (e) => {
        const p = e.payload;
        if (p.type === "enter" || p.type === "over") setOver(true);
        else if (p.type === "leave") setOver(false);
        else if (p.type === "drop") {
          setOver(false);
          const infos: DropInfo[] = [];
          for (const path of p.paths.slice(0, 20)) {
            try {
              infos.push(await api.analyzeDrop(path));
            } catch (err) {
              toast(errText(err), "error");
            }
          }
          setQueue((q) => [...q, ...infos]);
        }
      })
      .then((f) => (off = f))
      .catch(() => {});
    return () => off?.();
  }, [toast]);

  const cur = queue[0];
  useEffect(() => {
    setShowAll(false);
    setTarget(cur?.suggested[0] ?? "");
  }, [cur]);

  const next = useCallback(() => setQueue((q) => q.slice(1)), []);

  if (over) return <div className="dropzone">{t("drop.over")}</div>;
  if (!cur) return null;

  const fit = instances.filter((i) => cur.suggested.includes(i.id));
  const list = showAll ? instances : fit;

  const put = async () => {
    try {
      if (cur.kind === "modpack") {
        const inst = await api.importPack(cur.path);
        await reloadInstances();
        select(inst.id);
        toast(t("builds.imported", { name: inst.name }), "ok");
      } else {
        await api.placeDrop(cur.path, target, cur.kind);
        const b = instances.find((i) => i.id === target);
        sfx.success();
        toast(t("drop.done", { name: cur.name, build: b?.name ?? "" }), "ok");
      }
    } catch (e) {
      toast(errText(e), "error");
    }
    next();
  };

  return (
    <Modal title={t("drop.title")} onClose={next}>
      <div className="drop-file">
        <span className="mod-card__icon mod-card__icon--small">
          <Icon name={cur.kind === "world" ? "globe" : cur.kind === "modpack" ? "builds" : cur.kind === "shader" ? "sun" : cur.kind === "resourcepack" ? "image" : "mods"} size={20} />
        </span>
        <span className="content-row__text">
          <strong className="ellipsis">{cur.name}</strong>
          <span className="muted small">
            {t(`drop.kind.${cur.kind}` as Key)}
            {cur.loaders.length > 0 && ` · ${cur.loaders.map(loaderLabel).join(", ")}`}
            {cur.gameVersions.length > 0 && ` · ${t("drop.for", { list: cur.gameVersions.slice(-3).join(", ") })}`}
          </span>
        </span>
      </div>
      {cur.kind !== "modpack" && cur.kind !== "unknown" && (
        <>
          {fit.length === 0 && !showAll && <p className="notice">{t("drop.noFit")}</p>}
          <div className="drop-builds">
            {list.map((i) => (
              <label key={i.id} className={`friend__item ${target === i.id ? "is-on" : ""}`}>
                <input type="radio" name="drop-target" checked={target === i.id} onChange={() => setTarget(i.id)} />
                <span className="content-row__text">
                  <strong className="ellipsis">{i.name}</strong>
                  <span className="muted small">
                    {loaderLabel(i.loader)} · {i.mcVersion}
                  </span>
                </span>
              </label>
            ))}
          </div>
          {!showAll && instances.length > fit.length && (
            <button className="link small" onClick={() => setShowAll(true)}>
              {t("drop.showAll")}
            </button>
          )}
        </>
      )}
      <div className="modal__foot">
        <button className="btn" onClick={next}>
          {t("create.cancel")}
        </button>
        <button className="btn btn--primary" disabled={cur.kind === "unknown" || (cur.kind !== "modpack" && !target)} onClick={put}>
          {cur.kind === "modpack" ? t("builds.import") : t("drop.put")}
        </button>
      </div>
    </Modal>
  );
}
