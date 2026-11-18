import { useCallback, useEffect, useState } from "react";
import { api, errText, type ContentKind, type ContentUpdate, type ContentVersion, type Hit, type InstalledContent } from "../api";
import { useStore } from "../store";
import { Toggle, loaderLabel } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";
import { BuildDialog } from "./BuildDialog";
import { ModListExport } from "./ModList";
import type { Key } from "../i18n";

const kinds: ContentKind[] = ["mod", "resourcepack", "shader"];
const sorts = ["relevance", "downloads", "updated", "newest"] as const;
const kindFolder: Record<ContentKind, string> = { mod: "mods", resourcepack: "resourcepacks", shader: "shaderpacks" };

function fmt(n: number, lang: string) {
  return new Intl.NumberFormat(lang === "ru" ? "ru-RU" : "en-US", { notation: "compact" }).format(n);
}

async function packIds(instanceId: string): Promise<string[]> {
  try {
    const r = await api.linkSend(instanceId, { cmd: "listPacks" });
    return ((r.packs as { available?: string[] })?.available ?? []) as string[];
  } catch {
    return [];
  }
}

function ResultCard({ hit, instanceId, kind, installed, onInstalled }: { hit: Hit; instanceId: string; kind: ContentKind; installed: boolean; onInstalled: () => void }) {
  const { t, toast, settings, linked } = useStore();
  const [busy, setBusy] = useState(false);
  const [versions, setVersions] = useState<ContentVersion[] | null>(null);
  const [versionId, setVersionId] = useState("");
  const install = async () => {
    setBusy(true);
    try {
      // with the game running and linked, a new resource pack is switched on right away
      const live = kind === "resourcepack" && instanceId in linked && settings.companion && settings.link.packsLive;
      const before = live ? await packIds(instanceId) : [];
      const list = await api.installContent(instanceId, kind, hit.projectId, versionId || null);
      sfx.success();
      toast(t("mods.installedOk", { list: list.join(", ") }), "ok");
      onInstalled();
      if (live) {
        const added = (await packIds(instanceId)).filter((id) => !before.includes(id) && id.startsWith("file/"));
        for (const id of added) await api.linkSend(instanceId, { cmd: "enablePack", pack: id });
        if (added.length) toast(settings.lang === "ru" ? "Ресурспак включён прямо в игре" : "Resource pack switched on in the game", "ok");
      }
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="card mod-card">
      <span className="mod-card__icon">{hit.iconUrl ? <img src={hit.iconUrl} alt="" loading="lazy" /> : <Icon name="mods" size={28} />}</span>
      <div className="mod-card__body">
        <div className="mod-card__top">
          {hit.projectId.startsWith("own:") ? (
            <strong className="ellipsis">{hit.title}</strong>
          ) : (
            <button className="title-link ellipsis" title={t("mods.onModrinth")} onClick={() => void api.openUrl(`https://modrinth.com/project/${hit.slug}`)}>
              {hit.title}
            </button>
          )}
          <span className="muted small">{hit.author}</span>
        </div>
        <p className="muted small clamp">{hit.description}</p>
        <div className="mod-card__foot">
          {hit.projectId.startsWith("own:") ? (
            <span className="pill pill--ok">{t("mods.ownBadge")}</span>
          ) : (
            <span className="muted small">{t("mods.downloads", { n: fmt(hit.downloads, settings.lang) })}</span>
          )}
          <span className="grow" />
          {versions && (
            <select className="select-small" value={versionId} onChange={(e) => setVersionId(e.target.value)} aria-label={t("mods.versions")}>
              <option value="">{t("mods.latest")}</option>
              {versions.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.versionNumber}
                </option>
              ))}
            </select>
          )}
          {!versions && !installed && (
            <button
              className="icon-btn"
              title={t("mods.versions")}
              aria-label={t("mods.versions")}
              onClick={async () => {
                try {
                  setVersions(await api.contentVersions(instanceId, hit.projectId, kind));
                } catch (e) {
                  toast(errText(e), "error");
                }
              }}
            >
              <Icon name="chevron" size={16} />
            </button>
          )}
          {installed ? (
            <span className="pill pill--ok">{t("mods.installedBadge")}</span>
          ) : (
            <button className="btn btn--primary btn--small" disabled={busy} onClick={install}>
              {busy ? t("mods.loading") : t("mods.install")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export function Mods() {
  const { t, instances, selected, settings, updateSettings, toast, modsSearch } = useStore();
  const [instanceId, setInstanceId] = useState(selected?.id ?? "");
  const inst = instances.find((i) => i.id === instanceId) ?? null;
  const [kind, setKind] = useState<ContentKind>(inst?.loader === "vanilla" ? "resourcepack" : "mod");
  const [tab, setTab] = useState<"installed" | "browse">(modsSearch ? "browse" : "installed");
  const [installed, setInstalled] = useState<InstalledContent[] | null>(null);
  const [exporting, setExporting] = useState(false);
  const [query, setQuery] = useState(modsSearch ?? "");
  const [sort, setSort] = useState<(typeof sorts)[number]>("relevance");
  const [hits, setHits] = useState<Hit[]>([]);
  const [total, setTotal] = useState(0);
  const [searching, setSearching] = useState(false);
  const [creatingBuild, setCreatingBuild] = useState(false);
  const [updates, setUpdates] = useState<ContentUpdate[] | null>(null);
  const [updating, setUpdating] = useState(false);
  const [optimizing, setOptimizing] = useState(false);

  const loadInstalled = useCallback(async () => {
    if (!inst) return;
    try {
      setInstalled(await api.listContent(inst.id, kind));
    } catch (e) {
      toast(errText(e), "error");
    }
  }, [inst, kind, toast]);

  useEffect(() => {
    setInstalled(null);
    setUpdates(null);
    void loadInstalled();
  }, [loadInstalled]);

  const search = useCallback(
    async (offset: number) => {
      if (!inst) return;
      setSearching(true);
      try {
        const r = await api.search(query, kind, inst.mcVersion, inst.loader, offset, sort);
        setHits((h) => (offset === 0 ? r.hits : [...h, ...r.hits]));
        setTotal(r.total);
      } catch (e) {
        toast(errText(e), "error");
      } finally {
        setSearching(false);
      }
    },
    [inst, kind, query, sort, toast],
  );

  useEffect(() => {
    if (tab !== "browse") return;
    const id = setTimeout(() => void search(0), 300);
    return () => clearTimeout(id);
  }, [tab, search]);

  if (!inst) {
    return (
      <div className="page page--center">
        <div className="soon">
          <span className="soon__icon">
            <Icon name="mods" size={40} />
          </span>
          <h1>{t("mods.title")}</h1>
          <p>{t("mods.noBuild")}</p>
          <button className="btn btn--primary" onClick={() => setCreatingBuild(true)}>
            <Icon name="plus" size={18} /> {t("home.newBuild")}
          </button>
          {creatingBuild && <BuildDialog onClose={() => setCreatingBuild(false)} />}
        </div>
      </div>
    );
  }

  const isVanilla = inst.loader === "vanilla";
  const installedIds = new Set((installed ?? []).map((i) => i.projectId).filter(Boolean));

  return (
    <div className="page">
      <div className="page-head">
        <h1>{t("mods.title")}</h1>
        <div className="page-head__tools">
          <select value={inst.id} onChange={(e) => setInstanceId(e.target.value)} aria-label={t("mods.build")}>
            {instances.map((i) => (
              <option key={i.id} value={i.id}>
                {i.name} · {loaderLabel(i.loader)} {i.mcVersion}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="tabs-row">
        <div className="seg seg--panel">
          {kinds.map((k) => (
            <button
              key={k}
              className={`seg__btn ${kind === k ? "seg__btn--on" : ""}`}
              disabled={k === "mod" && isVanilla}
              onClick={() => {
                sfx.click();
                setKind(k);
              }}
            >
              {t(`mods.kind.${k}` as Key)}
            </button>
          ))}
        </div>
        <div className="seg seg--panel">
          {(["installed", "browse"] as const).map((x) => (
            <button key={x} className={`seg__btn ${tab === x ? "seg__btn--on" : ""}`} onClick={() => setTab(x)}>
              {x === "installed" ? t("mods.installed") : t("mods.browse")}
            </button>
          ))}
        </div>
      </div>

      {isVanilla && kind === "mod" && <p className="notice">{t("mods.vanillaHint")}</p>}
      {kind === "shader" && <p className="notice">{t("mods.shaderHint")}</p>}

      {tab === "installed" ? (
        <>
          <div className="toolbar">
            <button
              className="btn"
              disabled={updating}
              onClick={async () => {
                setUpdating(true);
                try {
                  const u = await api.checkUpdates(inst.id);
                  setUpdates(u);
                  if (u.length === 0) toast(t("mods.noUpdates"), "ok");
                } catch (e) {
                  toast(errText(e), "error");
                } finally {
                  setUpdating(false);
                }
              }}
            >
              {t("mods.checkUpdates")}
            </button>
            {updates && updates.length > 0 && (
              <button
                className="btn btn--primary"
                disabled={updating}
                onClick={async () => {
                  setUpdating(true);
                  try {
                    const n = await api.applyUpdates(inst.id, []);
                    sfx.success();
                    toast(t("mods.updated", { n }), "ok");
                    setUpdates([]);
                    await loadInstalled();
                  } catch (e) {
                    toast(errText(e), "error");
                  } finally {
                    setUpdating(false);
                  }
                }}
              >
                {t("mods.updateAll", { n: updates.length })}
              </button>
            )}
            {!isVanilla && kind === "mod" && (
              <button
                className="btn"
                title={t("mods.optimizeHint")}
                disabled={optimizing}
                onClick={async () => {
                  setOptimizing(true);
                  try {
                    const r = await api.optimizeInstance(inst.id);
                    if (r.installed.length) {
                      sfx.success();
                      toast(t("mods.optimized", { list: r.installed.join(", ") }), "ok");
                    } else toast(t("mods.alreadyOptimized"), "ok");
                    await loadInstalled();
                  } catch (e) {
                    toast(errText(e), "error");
                  } finally {
                    setOptimizing(false);
                  }
                }}
              >
                <Icon name="rocket" size={16} /> {optimizing ? t("mods.optimizing") : t("mods.optimize")}
              </button>
            )}
            <button className="btn" onClick={() => void api.openInstanceFolder(inst.id, kindFolder[kind]).catch((e) => toast(errText(e), "error"))}>
              <Icon name="folder" size={16} /> {t("mods.openFolder")}
            </button>
            {installed && installed.length > 0 && (
              <button className="btn" onClick={() => setExporting(true)} title={settings.lang === "en" ? "Copy or save the list — for a friend or another launcher" : "Скопировать или сохранить список — для друга или другого лаунчера"}>
                <Icon name="link" size={16} /> {settings.lang === "en" ? "Share list" : "Список"}
              </button>
            )}
            {exporting && installed && <ModListExport inst={inst} kind={kind} items={installed} onClose={() => setExporting(false)} />}
            {installed && installed.length > 1 && (
              <>
                <button
                  className="btn btn--small"
                  onClick={async () => {
                    for (const c of installed.filter((x) => !x.enabled)) await api.toggleContent(inst.id, kind, c.filename, true).catch(() => {});
                    await loadInstalled();
                  }}
                >
                  {t("mods.enableAll")}
                </button>
                <button
                  className="btn btn--small"
                  onClick={async () => {
                    for (const c of installed.filter((x) => x.enabled)) await api.toggleContent(inst.id, kind, c.filename, false).catch(() => {});
                    await loadInstalled();
                  }}
                >
                  {t("mods.disableAll")}
                </button>
              </>
            )}
            <span className="grow" />
            <label className="check-inline">
              {t("mods.autoUpdate")}
              <Toggle on={settings.autoUpdateMods} onChange={(v) => updateSettings({ autoUpdateMods: v })} label={t("mods.autoUpdate")} />
            </label>
          </div>
          {updates && updates.length > 0 && (
            <div className="update-list">
              {updates.map((u) => (
                <div key={u.filename} className="update-row">
                  <strong>{u.title}</strong>
                  <span className="muted small">
                    {u.from} → {u.to}
                  </span>
                </div>
              ))}
            </div>
          )}
          {installed === null && <p className="muted">{t("mods.loading")}</p>}
          {installed?.length === 0 && <p className="empty">{t("mods.empty")}</p>}
          <div className="content-list">
            {installed?.map((c) => (
              <div key={c.filename} className={`content-row ${c.enabled ? "" : "content-row--off"}`}>
                <span className="mod-card__icon mod-card__icon--small">{c.icon ? <img src={c.icon} alt="" loading="lazy" /> : <Icon name="mods" size={20} />}</span>
                <span className="content-row__text">
                  {c.projectId && !c.projectId.startsWith("own:") ? (
                    <button className="title-link ellipsis" title={t("mods.onModrinth")} onClick={() => void api.openUrl(`https://modrinth.com/project/${c.projectId}`)}>
                      {c.title}
                    </button>
                  ) : (
                    <strong className="ellipsis">{c.title}</strong>
                  )}
                  <span className="muted small ellipsis">
                    {c.versionNumber ? `${c.versionNumber} · ` : ""}
                    {c.filename}
                  </span>
                </span>
                <Toggle
                  on={c.enabled}
                  label={c.title}
                  onChange={async (v) => {
                    try {
                      await api.toggleContent(inst.id, kind, c.filename, v);
                      await loadInstalled();
                    } catch (e) {
                      toast(errText(e), "error");
                    }
                  }}
                />
                <button
                  className="icon-btn icon-btn--danger"
                  aria-label={t("mods.remove")}
                  title={t("mods.remove")}
                  onClick={async () => {
                    try {
                      await api.removeContent(inst.id, kind, c.filename);
                      await loadInstalled();
                    } catch (e) {
                      toast(errText(e), "error");
                    }
                  }}
                >
                  <Icon name="trash" size={18} />
                </button>
              </div>
            ))}
          </div>
        </>
      ) : (
        <>
          <div className="toolbar">
            <input className="search search--wide" value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("mods.search")} aria-label={t("mods.search")} />
            <select value={sort} onChange={(e) => setSort(e.target.value as (typeof sorts)[number])} aria-label="Sort">
              {sorts.map((s) => (
                <option key={s} value={s}>
                  {t(`mods.sort.${s}` as Key)}
                </option>
              ))}
            </select>
          </div>
          {!searching && hits.length === 0 && <p className="empty">{t("mods.nothing")}</p>}
          <div className="grid grid--mods">
            {hits.map((h) => (
              <ResultCard
                key={h.projectId}
                hit={h}
                instanceId={inst.id}
                kind={kind}
                installed={installedIds.has(h.projectId)}
                onInstalled={() => void loadInstalled()}
              />
            ))}
          </div>
          {hits.length < total && (
            <button className="btn more-btn" disabled={searching} onClick={() => void search(hits.length)}>
              {searching ? t("mods.loading") : t("mods.more")}
            </button>
          )}
        </>
      )}
    </div>
  );
}
