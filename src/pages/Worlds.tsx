import { useCallback, useEffect, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { api, errText, fileSrc, type Backup, type Instance, type World } from "../api";
import { useStore } from "../store";
import { Modal, formatPlaytime } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";

function size(n: number) {
  if (n > 1024 * 1024 * 1024) return `${(n / 1024 ** 3).toFixed(1)} GB`;
  if (n > 1024 * 1024) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(n / 1024))} KB`;
}

export function WorldsDialog({ inst, onClose }: { inst: Instance; onClose: () => void }) {
  const { t, toast, settings } = useStore();
  const [worlds, setWorlds] = useState<World[] | null>(null);
  const [backups, setBackups] = useState<Backup[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [times, setTimes] = useState<Record<string, { seconds: number }>>({});
  useEffect(() => {
    api.worldTimes(inst.id).then(setTimes).catch(() => setTimes({}));
  }, [inst.id]);

  const load = useCallback(async () => {
    try {
      setWorlds(await api.listWorlds(inst.id));
      setBackups(await api.listBackups(inst.id));
    } catch (e) {
      toast(errText(e), "error");
    }
  }, [inst.id, toast]);
  useEffect(() => void load(), [load]);

  const when = (sec: number) =>
    new Date(sec * 1000).toLocaleString(settings.lang === "ru" ? "ru-RU" : "en-GB", { dateStyle: "medium", timeStyle: "short" });

  return (
    <Modal title={`${t("worlds.title")} · ${inst.name}`} onClose={onClose} wide>
      <h3 className="section-title">{t("worlds.worlds")}</h3>
      {worlds?.length === 0 && <p className="muted">{t("worlds.none")}</p>}
      <div className="content-list">
        {worlds?.map((w) => (
          <div key={w.name} className="content-row">
            <span className="mod-card__icon mod-card__icon--small">
              {w.icon ? <img src={fileSrc(w.icon) ?? ""} alt="" style={{ imageRendering: "pixelated" }} /> : <Icon name="image" size={20} />}
            </span>
            <span className="content-row__text">
              <strong className="ellipsis">{w.name}</strong>
              <span className="muted small">
                {size(w.size)} · {when(w.modified)}
                {times[`sp:${w.name}`]?.seconds ? ` · ${settings.lang === "ru" ? "в мире" : "played"} ${formatPlaytime(times[`sp:${w.name}`].seconds, settings.lang)}` : ""}
              </span>
            </span>
            <button
              className="btn btn--small btn--primary"
              disabled={busy !== null}
              onClick={async () => {
                setBusy(w.name);
                try {
                  await api.backupWorld(inst.id, w.name);
                  sfx.success();
                  toast(t("worlds.done"), "ok");
                  await load();
                } catch (e) {
                  toast(errText(e), "error");
                } finally {
                  setBusy(null);
                }
              }}
            >
              {busy === w.name ? t("mods.loading") : t("worlds.backup")}
            </button>
          </div>
        ))}
      </div>

      <div className="section-head section-head--tight">
        <h3 className="section-title">{t("worlds.backups")}</h3>
        <button className="link" onClick={() => void api.openInstanceFolder(inst.id, "backups").catch((e) => toast(errText(e), "error"))}>
          {t("worlds.openFolder")}
        </button>
      </div>
      {backups.length === 0 && <p className="muted">{t("worlds.noBackups")}</p>}
      <div className="content-list">
        {backups.map((b) => (
          <div key={b.file} className="content-row">
            <span className="content-row__text">
              <strong className="ellipsis">{b.world}</strong>
              <span className="muted small">
                {when(b.created)} · {size(b.size)}
              </span>
            </span>
            <button
              className="btn btn--small"
              disabled={busy !== null}
              onClick={async () => {
                if (!(await confirm(t("worlds.restoreConfirm", { world: b.world }), { title: "AniLauncher", kind: "warning" }))) return;
                setBusy(b.file);
                try {
                  await api.restoreBackup(inst.id, b.file);
                  sfx.success();
                  toast(t("worlds.restored"), "ok");
                  await load();
                } catch (e) {
                  toast(errText(e), "error");
                } finally {
                  setBusy(null);
                }
              }}
            >
              {t("worlds.restore")}
            </button>
            <button
              className="icon-btn icon-btn--danger"
              aria-label={t("worlds.delete")}
              title={t("worlds.delete")}
              onClick={async () => {
                try {
                  await api.deleteBackup(inst.id, b.file);
                  await load();
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
    </Modal>
  );
}
