import { useCallback, useEffect, useState } from "react";
import { api, errText, type GameVersion, type Hit } from "../api";
import { useStore } from "../store";
import { Modal } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";
import type { Key } from "../i18n";

const sorts = ["relevance", "downloads", "updated", "newest"] as const;

/** Modrinth modpack catalog: one click creates a ready build. */
export function Modpacks({ onClose }: { onClose: () => void }) {
  const { t, toast, reloadInstances, select, settings, go } = useStore();
  const [query, setQuery] = useState("");
  const [mc, setMc] = useState("");
  const [loader, setLoader] = useState("");
  const [sort, setSort] = useState<(typeof sorts)[number]>("downloads");
  const [versions, setVersions] = useState<GameVersion[]>([]);
  const [hits, setHits] = useState<Hit[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);

  useEffect(() => {
    api.listGameVersions("vanilla").then((v) => setVersions(v.filter((x) => x.kind === "release"))).catch(() => {});
  }, []);

  const search = useCallback(
    async (offset: number) => {
      setLoading(true);
      try {
        const r = await api.search(query, "modpack", mc || null, loader || null, offset, sort);
        setHits((h) => (offset === 0 ? r.hits : [...h, ...r.hits]));
        setTotal(r.total);
      } catch (e) {
        toast(errText(e), "error");
      } finally {
        setLoading(false);
      }
    },
    [query, mc, loader, sort, toast],
  );

  useEffect(() => {
    const id = setTimeout(() => void search(0), 300);
    return () => clearTimeout(id);
  }, [search]);

  const install = async (h: Hit) => {
    setInstalling(h.projectId);
    try {
      const inst = await api.installModpack(h.projectId);
      await reloadInstances();
      select(inst.id);
      sfx.success();
      toast(t("mp.installed", { name: inst.name }), "ok");
      onClose();
      go("home");
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setInstalling(null);
    }
  };

  const fmt = (n: number) => new Intl.NumberFormat(settings.lang === "ru" ? "ru-RU" : "en-US", { notation: "compact" }).format(n);

  return (
    <Modal title={t("mp.title")} onClose={onClose} wide xl>
      <div className="toolbar">
        <input className="search search--wide" value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("mp.search")} aria-label={t("mp.search")} autoFocus />
        <select value={mc} onChange={(e) => setMc(e.target.value)} aria-label={t("mp.version")}>
          <option value="">{t("mp.anyVersion")}</option>
          {versions.slice(0, 60).map((v) => (
            <option key={v.id} value={v.id}>
              {v.id}
            </option>
          ))}
        </select>
        <select value={loader} onChange={(e) => setLoader(e.target.value)} aria-label={t("create.loader")}>
          <option value="">{t("mp.anyLoader")}</option>
          <option value="fabric">Fabric</option>
          <option value="quilt">Quilt</option>
          <option value="neoforge">NeoForge</option>
          <option value="forge">Forge</option>
        </select>
        <select value={sort} onChange={(e) => setSort(e.target.value as (typeof sorts)[number])} aria-label="Sort">
          {sorts.map((s) => (
            <option key={s} value={s}>
              {t(`mods.sort.${s}` as Key)}
            </option>
          ))}
        </select>
      </div>
      {!loading && hits.length === 0 && <p className="empty">{t("mods.nothing")}</p>}
      <div className="grid grid--mods">
        {hits.map((h) => (
          <div key={h.projectId} className="card mod-card">
            <span className="mod-card__icon">{h.iconUrl ? <img src={h.iconUrl} alt="" loading="lazy" /> : <Icon name="builds" size={28} />}</span>
            <div className="mod-card__body">
              <div className="mod-card__top">
                <strong className="ellipsis">{h.title}</strong>
                <span className="muted small">{h.author}</span>
              </div>
              <p className="muted small clamp">{h.description}</p>
              <div className="mod-card__foot">
                <span className="muted small">{t("mods.downloads", { n: fmt(h.downloads) })}</span>
                <span className="grow" />
                <button className="link small" onClick={() => api.openUrl(`https://modrinth.com/modpack/${h.slug}`)}>
                  Modrinth
                </button>
                <button className="btn btn--primary btn--small" disabled={installing !== null} onClick={() => void install(h)}>
                  {installing === h.projectId ? t("mp.installing") : t("mp.install")}
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
      {hits.length < total && (
        <button className="btn more-btn" disabled={loading} onClick={() => void search(hits.length)}>
          {loading ? t("mods.loading") : t("mods.more")}
        </button>
      )}
    </Modal>
  );
}
