import { useEffect, useMemo, useRef, useState } from "react";
import { api, errText, type CrashFix, type CrashHint } from "../api";
import { useStore } from "../store";
import { Modal, Toggle } from "../components/Bits";
import { Icon } from "../components/Icon";
import { ReportDialog } from "./Care";
import type { Key } from "../i18n";

function level(line: string): "error" | "warn" | "info" {
  if (/\b(ERROR|FATAL|SEVERE)\b|Exception|^\s+at /.test(line)) return "error";
  if (/\bWARN(ING)?\b/.test(line)) return "warn";
  return "info";
}

function hintText(h: CrashHint, t: (k: Key, v?: Record<string, string | number>) => string, lang: "ru" | "en") {
  if (h.kind === "mixin" && !h.detail) return t("hint.mixinUnknown");
  const first = h.fixes?.[0];
  if (h.kind === "missingMod" && first?.action === "install") return t("hint.needMod", { name: first.name });
  if (h.kind === "missingMod" && first?.action === "update") return t("hint.oldMod", { name: first.name });
  let detail = h.detail;
  if (lang === "ru") {
    detail = detail
      .replace(/^Install (.+?), any version\.?$/, "поставь $1 (любую версию)")
      .replace(/^Install (.+?), version (.+?)\.?$/, "поставь $1 версии $2")
      .replace(/^Install /, "поставь ")
      .replace(/^Replace mod (.+?) with version (.+?)\.?$/, "замени мод $1 на версию $2")
      .replace(/^Replace /, "замени ");
  }
  return t(`hint.${h.kind}` as Key, { detail });
}

/** Live game output with filters, and an explanation when the game crashed. */
export function Console({ instanceId, onClose }: { instanceId: string; onClose: () => void }) {
  const { t, logs, setLog, crashes, instances, running, toast, settings, mask } = useStore();
  const inst = instances.find((i) => i.id === instanceId);
  const lines = logs[instanceId];
  const crash = crashes[instanceId];
  const [filter, setFilter] = useState("");
  const [problems, setProblems] = useState(false);
  const [report, setReport] = useState(false);
  const [auto, setAuto] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [done, setDone] = useState<string[]>([]);
  const box = useRef<HTMLDivElement>(null);

  // After a launcher restart the live buffer is empty: read the saved log once
  useEffect(() => {
    if (lines === undefined) api.readLaunchLog(instanceId).then((l) => setLog(instanceId, l)).catch(() => {});
  }, [instanceId, lines, setLog]);

  const shown = useMemo(() => {
    const f = filter.trim().toLowerCase();
    return (lines ?? []).filter((l) => (!f || l.toLowerCase().includes(f)) && (!problems || level(l) !== "info")).slice(-3000);
  }, [lines, filter, problems]);

  // a new crash (or a fresh start) resets the fixes already applied
  useEffect(() => setDone([]), [crash]);

  const runFix = async (f: CrashFix) => {
    const key = `${f.action}:${f.target}`;
    setBusy(key);
    try {
      const msg = await api.crashFix(instanceId, f.action, f.target);
      toast(msg, "ok");
      setDone((d) => [...d, key]);
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(null);
    }
  };

  useEffect(() => {
    if (auto && box.current) box.current.scrollTop = box.current.scrollHeight;
  }, [shown, auto]);

  return (
    <Modal title={`${t("console.title")}${inst ? ` · ${inst.name}` : ""}`} onClose={onClose} wide xl>
      {crash && (
        <div className="crash">
          <strong>{t("console.crashTitle")}</strong>
          {crash.hints.length === 0 && <p className="muted">{t("console.noHints")}</p>}
          <ul>
            {crash.hints.map((h, i) => (
              <li key={i}>
                {hintText(h, t, settings.lang)}
                {(h.fixes ?? []).length > 0 && (
                  <div className="crash__fixes">
                    {(h.fixes ?? []).map((f) => {
                      const key = `${f.action}:${f.target}`;
                      return (
                        <button
                          key={key}
                          className={`btn btn--small${f.action === "disable" ? "" : " btn--primary"}`}
                          disabled={busy !== null || done.includes(key)}
                          onClick={() => runFix(f)}
                        >
                          {done.includes(key) ? <Icon name="check" size={14} /> : null}
                          {busy === key ? "…" : t(`fix.${f.action}` as Key, { name: f.name })}
                        </button>
                      );
                    })}
                  </div>
                )}
              </li>
            ))}
          </ul>
          {done.length > 0 && !running.includes(instanceId) && (
            <div className="crash__again">
              <span>{t("fix.done")}</span>
              <button className="btn btn--small btn--primary" onClick={() => api.launchInstance(instanceId).catch((e) => toast(errText(e), "error"))}>
                <Icon name="play" size={14} /> {t("fix.again")}
              </button>
            </div>
          )}
          {crash.crashReport && (
            <button className="btn btn--small" onClick={() => api.openFile(crash.crashReport!).catch((e) => toast(errText(e), "error"))}>
              {t("console.report")}
            </button>
          )}
        </div>
      )}
      <div className="toolbar">
        <input className="search search--wide" value={filter} onChange={(e) => setFilter(e.target.value)} placeholder={t("console.filter")} aria-label={t("console.filter")} />
        <label className="check-inline">
          {t("console.problems")}
          <Toggle on={problems} onChange={setProblems} label={t("console.problems")} />
        </label>
        <label className="check-inline">
          {t("console.autoscroll")}
          <Toggle on={auto} onChange={setAuto} label={t("console.autoscroll")} />
        </label>
        <button
          className="btn btn--small"
          onClick={async () => {
            await navigator.clipboard.writeText((lines ?? []).map(mask).join("\n"));
            toast(t("console.copied"), "ok");
          }}
        >
          <Icon name="copy" size={16} /> {t("console.copy")}
        </button>
        <button className="btn btn--small" onClick={() => api.openInstanceFolder(instanceId, "logs").catch((e) => toast(errText(e), "error"))}>
          <Icon name="folder" size={16} /> {t("console.folder")}
        </button>
        <button className="btn btn--small" onClick={() => setReport(true)}>
          <Icon name="download" size={16} /> {settings.lang === "ru" ? "Отчёт об ошибке" : "Bug report"}
        </button>
        {running.includes(instanceId) && (
          <button className="btn btn--small btn--danger" onClick={() => api.stopInstance(instanceId).catch((e) => toast(errText(e), "error"))}>
            <Icon name="stop" size={14} /> {t("home.stop")}
          </button>
        )}
      </div>
      <div className="console" ref={box} onScroll={(e) => {
        const el = e.currentTarget;
        const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 30;
        if (!atBottom && auto) setAuto(false);
      }}>
        {shown.length === 0 && <span className="muted">{t("console.empty")}</span>}
        {shown.map((l, i) => (
          <div key={i} className={`console__line console__line--${level(l)}`}>
            {mask(l)}
          </div>
        ))}
      </div>
      {report && <ReportDialog instanceId={instanceId} onClose={() => setReport(false)} />}
    </Modal>
  );
}
