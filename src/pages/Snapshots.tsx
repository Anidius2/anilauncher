import { useCallback, useEffect, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { api, errText, type Instance, type Snapshot } from "../api";
import { useStore } from "../store";
import { Modal } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";
import type { Key } from "../i18n";

/** Restore points of a build: a time machine for mods and configs. */
export function SnapshotsDialog({ inst, onClose }: { inst: Instance; onClose: () => void }) {
  const { t, toast, settings, reloadInstances } = useStore();
  const [list, setList] = useState<Snapshot[] | null>(null);
  const [label, setLabel] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setList(await api.listSnapshots(inst.id));
    } catch (e) {
      toast(errText(e), "error");
    }
  }, [inst.id, toast]);
  useEffect(() => void load(), [load]);

  const when = (s: number) =>
    new Date(s * 1000).toLocaleString(settings.lang === "ru" ? "ru-RU" : "en-GB", { dateStyle: "medium", timeStyle: "short" });
  const size = (n: number) => (n > 1024 * 1024 ? `${(n / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1024))} KB`);

  return (
    <Modal title={`${t("snap.title")} · ${inst.name}`} onClose={onClose} wide>
      <p className="muted small">{t("snap.hint")}</p>
      <div className="inline-form">
        <input value={label} maxLength={60} placeholder={t("snap.labelPh")} onChange={(e) => setLabel(e.target.value)} aria-label={t("snap.labelPh")} />
        <button
          className="btn btn--primary"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              await api.createSnapshot(inst.id, label);
              setLabel("");
              sfx.success();
              toast(t("snap.created"), "ok");
              await load();
            } catch (e) {
              toast(errText(e), "error");
            } finally {
              setBusy(false);
            }
          }}
        >
          <Icon name="plus" size={16} /> {t("snap.create")}
        </button>
      </div>
      {list?.length === 0 && <p className="empty">{t("snap.empty")}</p>}
      <div className="timeline">
        {list?.map((s) => (
          <div key={s.file} className="timeline__item">
            <span className={`timeline__dot timeline__dot--${s.reason}`} />
            <span className="content-row__text">
              <strong>{s.label || t(`snap.reason.${s.reason}` as Key)}</strong>
              <span className="muted small">
                {when(s.created)} · {s.label ? `${t(`snap.reason.${s.reason}` as Key)} · ` : ""}
                {t("snap.mods", { n: s.mods })} · {size(s.size)}
              </span>
            </span>
            <button
              className="btn btn--small"
              disabled={busy}
              onClick={async () => {
                if (!(await confirm(t("snap.restoreConfirm"), { title: "AniLauncher", kind: "warning" }))) return;
                setBusy(true);
                try {
                  await api.restoreSnapshot(inst.id, s.file);
                  sfx.success();
                  toast(t("snap.restored"), "ok");
                  await reloadInstances();
                  await load();
                } catch (e) {
                  toast(errText(e), "error");
                } finally {
                  setBusy(false);
                }
              }}
            >
              <Icon name="undo" size={14} /> {t("snap.restore")}
            </button>
            <button
              className="icon-btn icon-btn--danger"
              aria-label={t("worlds.delete")}
              title={t("worlds.delete")}
              onClick={async () => {
                await api.deleteSnapshot(inst.id, s.file).catch((e) => toast(errText(e), "error"));
                await load();
              }}
            >
              <Icon name="trash" size={16} />
            </button>
          </div>
        ))}
      </div>
    </Modal>
  );
}
