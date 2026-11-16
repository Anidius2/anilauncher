import { useCallback, useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { confirm } from "../components/Confirm";
import { api, errText, fileSrc, type Shot, type ShotMeta } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";

export function Screenshots() {
  const { t, toast, settings, instances } = useStore();
  const [shots, setShots] = useState<Shot[] | null>(null);
  const [filter, setFilter] = useState("");
  const [open, setOpen] = useState<number | null>(null);

  const [meta, setMeta] = useState<Record<string, ShotMeta>>({});
  const load = useCallback(async () => {
    try {
      setShots(await api.listScreenshots());
      setMeta(await api.screenshotMeta().catch(() => ({})));
    } catch (e) {
      toast(errText(e), "error");
    }
  }, [toast]);
  useEffect(() => void load(), [load]);
  // a screenshot taken in the game appears here at once
  useEffect(() => {
    const on = () => void load();
    window.addEventListener("anilink-shot", on);
    return () => window.removeEventListener("anilink-shot", on);
  }, [load]);
  const DIM: Record<string, [string, string]> = {
    "minecraft:overworld": ["Верхний мир", "Overworld"],
    "minecraft:the_nether": ["Незер", "Nether"],
    "minecraft:the_end": ["Энд", "The End"],
  };
  const place = (s: Shot) => {
    const m = meta[s.path];
    if (!m?.world) return null;
    const world = m.kind === "multiplayer" && settings.streamer ? (settings.lang === "ru" ? "сервер" : "server") : m.world;
    const dim = m.dim ? (DIM[m.dim]?.[settings.lang === "ru" ? 0 : 1] ?? m.dim.replace(/^minecraft:/, "")) : "";
    const xyz = m.x !== undefined && !settings.streamer ? ` · ${m.x} ${m.y} ${m.z}` : "";
    return `${world}${dim ? ` · ${dim}` : ""}${xyz}`;
  };

  const list = (shots ?? []).filter((s) => !filter || s.instanceId === filter);
  const cur = open !== null ? list[open] : null;

  useEffect(() => {
    if (open === null) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(null);
      if (e.key === "ArrowRight") setOpen((o) => (o === null ? o : Math.min(list.length - 1, o + 1)));
      if (e.key === "ArrowLeft") setOpen((o) => (o === null ? o : Math.max(0, o - 1)));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, list.length]);

  const date = (s: Shot) =>
    new Date(s.modified * 1000).toLocaleString(settings.lang === "ru" ? "ru-RU" : "en-GB", { dateStyle: "medium", timeStyle: "short" });

  const copy = async (s: Shot) => {
    try {
      await api.copyImage(s.path);
      sfx.success();
      toast(t("shots.copied"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };
  const saveCopy = async (s: Shot) => {
    try {
      const dest = await save({ defaultPath: s.name, filters: [{ name: "Image", extensions: [s.name.split(".").pop() ?? "png"] }] });
      if (!dest) return;
      await api.saveCopy(s.path, dest);
      toast(t("shots.saved"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };
  const remove = async (s: Shot) => {
    if (settings.ui.confirmDelete && !(await confirm(t("shots.deleteConfirm"), { title: "AniLauncher", kind: "warning" }))) return;
    try {
      await api.deleteScreenshot(s.path);
      setOpen(null);
      await load();
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>{t("shots.title")}</h1>
        <div className="page-head__tools">
          <span className="muted small">{t("shots.count", { n: list.length })}</span>
          <select value={filter} onChange={(e) => setFilter(e.target.value)} aria-label={t("shots.all")}>
            <option value="">{t("shots.all")}</option>
            {instances.map((i) => (
              <option key={i.id} value={i.id}>
                {i.name}
              </option>
            ))}
          </select>
        </div>
      </div>
      {shots && list.length === 0 && <p className="empty">{t("shots.empty")}</p>}
      <div className="grid grid--shots">
        {list.map((s, i) => (
          <button
            key={s.path}
            className="card shot"
            onMouseEnter={sfx.hover}
            onClick={() => {
              sfx.open();
              setOpen(i);
            }}
          >
            <img src={fileSrc(s.path) ?? ""} alt={s.name} loading="lazy" />
            <span className="shot__meta">
              <span className="ellipsis strong">{s.instanceName}</span>
              <span className="small ellipsis">{place(s) ?? date(s)}</span>
            </span>
          </button>
        ))}
      </div>

      {cur && (
        <div className="lightbox" onMouseDown={(e) => e.target === e.currentTarget && setOpen(null)}>
          <button className="lightbox__nav lightbox__nav--prev" aria-label="←" disabled={open === 0} onClick={() => setOpen((o) => Math.max(0, (o ?? 0) - 1))}>
            ‹
          </button>
          <figure className="lightbox__fig">
            <img src={fileSrc(cur.path) ?? ""} alt={cur.name} />
            <figcaption>
              <span>
                <strong>{cur.instanceName}</strong> · {date(cur)}
                {place(cur) && <span className="shot__place"> · {place(cur)}</span>}
              </span>
              <span className="lightbox__actions">
                <button className="btn btn--small" onClick={() => void copy(cur)}>
                  <Icon name="copy" size={16} /> {t("shots.copy")}
                </button>
                <button className="btn btn--small" onClick={() => void saveCopy(cur)}>
                  <Icon name="folder" size={16} /> {t("shots.save")}
                </button>
                <button className="btn btn--small btn--danger" onClick={() => void remove(cur)}>
                  <Icon name="trash" size={16} /> {t("shots.delete")}
                </button>
                <button className="icon-btn" aria-label={t("win.close")} onClick={() => setOpen(null)}>
                  <Icon name="close" />
                </button>
              </span>
            </figcaption>
          </figure>
          <button
            className="lightbox__nav lightbox__nav--next"
            aria-label="→"
            disabled={open === list.length - 1}
            onClick={() => setOpen((o) => Math.min(list.length - 1, (o ?? 0) + 1))}
          >
            ›
          </button>
        </div>
      )}
    </div>
  );
}
