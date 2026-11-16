import { useState } from "react";
import { api, errText, type ModProblem } from "../api";
import { useStore } from "../store";
import { Modal } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";
import type { Key } from "../i18n";

export function problemText(p: ModProblem, t: (k: Key, v?: Record<string, string | number>) => string) {
  const range = p.detail && p.detail !== "*" ? ` (${p.detail})` : "";
  return t(`cf.${p.kind}` as Key, { mod: p.modName, other: p.other, detail: p.detail, file: p.modFile, range });
}

/** Shown before launch when the mod check finds something. */
export function ConflictDialog() {
  const { t, check, setCheck, play, toast, go, instances, searchMods } = useStore();
  const [busy, setBusy] = useState(false);
  if (!check) return null;
  const inst = instances.find((i) => i.id === check.id);
  const errors = check.problems.filter((p) => p.severity === "error");
  const warnings = check.problems.filter((p) => p.severity !== "error");

  const recheck = async () => {
    setBusy(true);
    try {
      const problems = await api.checkMods(check.id);
      if (problems.length === 0) {
        sfx.success();
        toast(t("cf.ok"), "ok");
        setCheck(null);
      } else setCheck({ id: check.id, problems });
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(false);
    }
  };

  const disable = async (file: string) => {
    try {
      await api.toggleContent(check.id, "mod", file, false);
      await recheck();
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const row = (p: ModProblem, i: number) => (
    <div key={i} className={`problem problem--${p.severity}`}>
      <Icon name={p.severity === "error" ? "close" : "sparkle"} size={16} />
      <span className="problem__text">{problemText(p, t)}</span>
      {p.kind === "missingDep" ? (
        <button className="btn btn--small" onClick={() => { setCheck(null); searchMods(p.other, check.id); }}>
          {t("cf.findMod", { name: p.other })}
        </button>
      ) : (
        <button className="btn btn--small" disabled={busy} onClick={() => void disable(p.modFile)}>
          {t("cf.disable")} {p.modFile.length > 28 ? `${p.modFile.slice(0, 26)}…` : p.modFile}
        </button>
      )}
    </div>
  );

  return (
    <Modal title={`${t("cf.title")}${inst ? ` · ${inst.name}` : ""}`} onClose={() => setCheck(null)} wide>
      {errors.length > 0 && (
        <>
          <strong className="problem-head problem-head--error">{t("cf.errors")}</strong>
          <div className="problem-list">{errors.map(row)}</div>
        </>
      )}
      {warnings.length > 0 && (
        <>
          <strong className="problem-head">{t("cf.warnings")}</strong>
          <div className="problem-list">{warnings.map(row)}</div>
        </>
      )}
      <div className="modal__foot">
        <button className="btn" onClick={() => { setCheck(null); go("mods"); }}>
          {t("cf.openMods")}
        </button>
        <button className="btn" disabled={busy} onClick={() => void recheck()}>
          {t("cf.recheck")}
        </button>
        <button className={`btn ${errors.length ? "btn--danger" : "btn--primary"}`} onClick={() => void play(check.id, true, check.quick)}>
          {t("cf.playAnyway")}
        </button>
      </div>
    </Modal>
  );
}
